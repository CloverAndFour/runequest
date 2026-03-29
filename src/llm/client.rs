//! xAI (Grok) client with function calling support.

use futures_util::Stream;
use reqwest::Client;
use std::pin::Pin;
use std::task::{Context, Poll};

use super::pricing::TokenUsage;
use super::types::*;
use crate::error::{RunequestError, Result};

const XAI_API_BASE: &str = "https://api.x.ai/v1";

pub struct XaiClient {
    client: Client,
    api_key: String,
    model: String,
}

impl XaiClient {
    pub fn new(api_key: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.into(),
            model: model.into(),
        }
    }

    pub fn default_model(&self) -> &str {
        &self.model
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Non-streaming chat with tools. Returns response + optional token usage.
    pub async fn chat_with_tools(
        &self,
        messages: &[ChatMessage],
        tools: &[ToolDef],
        model_override: Option<&str>,
    ) -> Result<(XaiResponse, Option<TokenUsage>)> {
        let model = model_override.unwrap_or(&self.model);

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "stream": false,
        });

        if !tools.is_empty() {
            body["tools"] = serde_json::to_value(tools)
                .map_err(|e| RunequestError::LlmError(e.to_string()))?;
        }

        let response = self
            .client
            .post(format!("{}/chat/completions", XAI_API_BASE))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| RunequestError::LlmError(format!("Network error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            if status.as_u16() == 429 {
                return Err(RunequestError::LlmRateLimited { retry_after_secs: 5 });
            }
            return Err(RunequestError::LlmError(format!(
                "xAI API returned {}: {}",
                status, text
            )));
        }

        let data: serde_json::Value = response
            .json()
            .await
            .map_err(|e| RunequestError::LlmError(format!("Parse error: {}", e)))?;

        // Extract usage
        let usage = data.get("usage").map(|u| TokenUsage {
            prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0),
            completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0),
        });

        let choice = data
            .get("choices")
            .and_then(|c| c.get(0))
            .ok_or_else(|| RunequestError::LlmError("No choices in response".to_string()))?;

        let message = choice
            .get("message")
            .ok_or_else(|| RunequestError::LlmError("No message in choice".to_string()))?;

        // Check for tool calls
        if let Some(tool_calls_val) = message.get("tool_calls") {
            if let Ok(tool_calls) = serde_json::from_value::<Vec<ToolCall>>(tool_calls_val.clone())
            {
                if !tool_calls.is_empty() {
                    let text = message
                        .get("content")
                        .and_then(|c| c.as_str())
                        .map(String::from);
                    return Ok((XaiResponse::ToolCalls { tool_calls, text }, usage));
                }
            }
        }

        let content = message
            .get("content")
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .to_string();
        Ok((XaiResponse::Text(content), usage))
    }

    /// Streaming chat — used for narrative output (no tools expected).
    pub async fn stream_chat(
        &self,
        messages: &[ChatMessage],
    ) -> Result<Pin<Box<dyn Stream<Item = Result<String>> + Send>>> {
        let body = serde_json::json!({
            "model": &self.model,
            "messages": messages,
            "stream": true,
        });

        let response = self
            .client
            .post(format!("{}/chat/completions", XAI_API_BASE))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| RunequestError::LlmError(format!("Network error: {}", e)))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            if status.as_u16() == 429 {
                return Err(RunequestError::LlmRateLimited { retry_after_secs: 5 });
            }
            return Err(RunequestError::LlmError(format!(
                "xAI API returned {}: {}",
                status, text
            )));
        }

        let byte_stream = response.bytes_stream();
        Ok(Box::pin(SseTextStream::new(byte_stream)))
    }
}

/// SSE stream parser that yields text chunks.
struct SseTextStream {
    inner: Pin<Box<dyn Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>> + Send>>,
    buffer: String,
    done: bool,
}

impl SseTextStream {
    fn new(
        stream: impl Stream<Item = std::result::Result<bytes::Bytes, reqwest::Error>> + Send + 'static,
    ) -> Self {
        Self {
            inner: Box::pin(stream),
            buffer: String::new(),
            done: false,
        }
    }
}

impl Stream for SseTextStream {
    type Item = Result<String>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }

        loop {
            if let Some(pos) = self.buffer.find('\n') {
                let line = self.buffer[..pos].to_string();
                self.buffer = self.buffer[pos + 1..].to_string();

                let line = line.trim_end_matches('\r');
                if line.is_empty() {
                    continue;
                }

                if let Some(data) = line.strip_prefix("data: ") {
                    let trimmed = data.trim();
                    if trimmed == "[DONE]" {
                        self.done = true;
                        return Poll::Ready(None);
                    }

                    if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(trimmed) {
                        if let Some(content) = parsed
                            .get("choices")
                            .and_then(|c| c.get(0))
                            .and_then(|c| c.get("delta"))
                            .and_then(|d| d.get("content"))
                            .and_then(|c| c.as_str())
                        {
                            if !content.is_empty() {
                                return Poll::Ready(Some(Ok(content.to_string())));
                            }
                        }
                    }
                    continue;
                }
                continue;
            }

            match self.inner.as_mut().poll_next(cx) {
                Poll::Ready(Some(Ok(bytes))) => {
                    if let Ok(text) = std::str::from_utf8(&bytes) {
                        self.buffer.push_str(text);
                    }
                }
                Poll::Ready(Some(Err(e))) => {
                    return Poll::Ready(Some(Err(RunequestError::LlmError(format!(
                        "Stream error: {}",
                        e
                    )))));
                }
                Poll::Ready(None) => {
                    self.done = true;
                    return Poll::Ready(None);
                }
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }
    }
}
