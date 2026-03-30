//! Internal wiki server — renders markdown from wiki/ as HTML on a separate port.
//! Bound to the headscale interface only (not public).

use anyhow::Result;
use axum::{
    extract::{Path, State},
    response::Html,
    routing::get,
    Router,
};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::TcpListener;

pub async fn run_wiki_server(
    port: u16,
    bind_address: &str,
    wiki_dir: PathBuf,
    tls_config: Option<axum_server::tls_rustls::RustlsConfig>,
) -> Result<()> {
    let wiki_dir = Arc::new(wiki_dir.canonicalize().unwrap_or(wiki_dir));

    let app = Router::new()
        .route("/", get(serve_index))
        .route("/*path", get(serve_page))
        .with_state(wiki_dir);

    let addr = format!("{}:{}", bind_address, port);

    if let Some(tls) = tls_config {
        eprintln!("Wiki server listening on https://{}", addr);
        let addr: std::net::SocketAddr = addr.parse()?;
        axum_server::bind_rustls(addr, tls)
            .serve(app.into_make_service())
            .await?;
    } else {
        eprintln!("Wiki server listening on http://{}", addr);
        let listener = TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;
    }
    Ok(())
}

async fn serve_index(State(wiki_dir): State<Arc<PathBuf>>) -> Html<String> {
    serve_md(&wiki_dir, "index").await
}

async fn serve_page(
    State(wiki_dir): State<Arc<PathBuf>>,
    Path(path): Path<String>,
) -> Html<String> {
    serve_md(&wiki_dir, &path).await
}

async fn serve_md(wiki_dir: &std::path::Path, path: &str) -> Html<String> {
    let clean = path.trim_matches('/');
    let clean = clean.strip_suffix(".md").unwrap_or(clean);

    let md_path = if clean.is_empty() {
        wiki_dir.join("index.md")
    } else {
        wiki_dir.join(format!("{}.md", clean))
    };

    // Path traversal guard
    if let Ok(canonical) = md_path.canonicalize() {
        if !canonical.starts_with(wiki_dir) {
            return Html(render_html("403 Forbidden", "<p>Access denied.</p>"));
        }
    } else {
        return Html(render_html(
            "404 Not Found",
            &format!(
                "<p>Page <code>{}</code> not found.</p><p><a href=\"/\">← Back to index</a></p>",
                clean
            ),
        ));
    }

    match tokio::fs::read_to_string(&md_path).await {
        Ok(content) => {
            let title = extract_title(&content);
            let body = markdown_to_html(&content);
            Html(render_html(&title, &body))
        }
        Err(_) => Html(render_html(
            "404 Not Found",
            &format!(
                "<p>Page <code>{}</code> not found.</p><p><a href=\"/\">← Back to index</a></p>",
                clean
            ),
        )),
    }
}

fn extract_title(md: &str) -> String {
    for line in md.lines() {
        let trimmed = line.trim();
        if let Some(title) = trimmed.strip_prefix("# ") {
            return title.to_string();
        }
    }
    "RuneQuest Wiki".to_string()
}

fn markdown_to_html(md: &str) -> String {
    use pulldown_cmark::{html, Options, Parser};
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    let parser = Parser::new_ext(md, options);
    let mut html_output = String::new();
    html::push_html(&mut html_output, parser);

    // Rewrite .md links to extensionless so wiki navigation works
    html_output
        .replace(".md\"", "\"")
        .replace(".md)", ")")
}

fn render_html(title: &str, body: &str) -> String {
    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{title} — RuneQuest Wiki</title>
<style>
* {{ margin: 0; padding: 0; box-sizing: border-box; }}
body {{
    font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    line-height: 1.6;
    color: #c9d1d9;
    background: #0d1117;
}}
nav {{
    background: #161b22;
    border-bottom: 1px solid #30363d;
    padding: 0.75rem 2rem;
    font-size: 0.9rem;
}}
nav a {{ color: #58a6ff; text-decoration: none; font-weight: 600; }}
nav a:hover {{ text-decoration: underline; }}
nav em {{ color: #f85149; font-style: normal; font-size: 0.8rem; }}
main {{
    max-width: 52rem;
    margin: 2rem auto;
    padding: 0 2rem;
}}
h1 {{ color: #f0f6fc; border-bottom: 1px solid #30363d; padding-bottom: 0.4rem; margin-bottom: 1rem; }}
h2 {{ color: #e6edf3; margin-top: 1.8rem; margin-bottom: 0.6rem; }}
h3 {{ color: #e6edf3; margin-top: 1.4rem; margin-bottom: 0.4rem; }}
a {{ color: #58a6ff; text-decoration: none; }}
a:hover {{ text-decoration: underline; }}
code {{
    background: #161b22;
    padding: 0.15rem 0.4rem;
    border-radius: 4px;
    font-size: 0.9em;
    color: #e6edf3;
}}
pre {{
    background: #161b22;
    border: 1px solid #30363d;
    border-radius: 6px;
    padding: 1rem;
    overflow-x: auto;
    margin: 1rem 0;
}}
pre code {{ background: none; padding: 0; }}
table {{
    border-collapse: collapse;
    width: 100%;
    margin: 1rem 0;
}}
th, td {{
    border: 1px solid #30363d;
    padding: 0.5rem 0.75rem;
    text-align: left;
}}
th {{ background: #161b22; color: #e6edf3; }}
blockquote {{
    border-left: 3px solid #f85149;
    padding: 0.5rem 1rem;
    margin: 1rem 0;
    background: #161b22;
    border-radius: 0 4px 4px 0;
}}
ul, ol {{ padding-left: 1.5rem; margin: 0.5rem 0; }}
li {{ margin: 0.25rem 0; }}
hr {{ border: none; border-top: 1px solid #30363d; margin: 1.5rem 0; }}
</style>
</head>
<body>
<nav><a href="/">RuneQuest Wiki</a> · <em>INTERNAL — CONFIDENTIAL</em></nav>
<main>
{body}
</main>
</body>
</html>"#,
        title = title,
        body = body,
    )
}
