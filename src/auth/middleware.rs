//! Auth middleware for axum routes.

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use std::sync::Arc;

use super::JwtManager;
use super::UserStore;

#[derive(Debug, Clone)]
pub struct AuthUser {
    pub username: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthMode {
    Disabled,
    Enabled,
}

pub struct AuthState {
    pub auth_mode: AuthMode,
    pub jwt_manager: Arc<JwtManager>,
    pub user_store: Arc<UserStore>,
}

pub async fn require_auth(
    State(auth_state): State<Arc<AuthState>>,
    mut req: Request,
    next: Next,
) -> Response {
    if auth_state.auth_mode == AuthMode::Disabled {
        req.extensions_mut().insert(AuthUser {
            username: "default".to_string(),
            role: "admin".to_string(),
        });
        return next.run(req).await;
    }

    let header_token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.to_string());

    let query_token = req.uri().query().and_then(|q| {
        q.split('&')
            .find_map(|pair| pair.strip_prefix("token="))
            .map(|t| t.to_string())
    });

    let token = match header_token.or(query_token) {
        Some(t) => t,
        None => return redirect_or_401(&req),
    };

    // Check if this is an API key (rq_ prefix) or a JWT
    if token.starts_with("rq_") {
        match auth_state.user_store.authenticate_api_key(&token) {
            Ok(user) => {
                req.extensions_mut().insert(AuthUser {
                    username: user.username,
                    role: user.role.to_string(),
                });
                next.run(req).await
            }
            Err(_) => redirect_or_401(&req),
        }
    } else {
        match auth_state.jwt_manager.validate_token(&token) {
            Ok(claims) => {
                req.extensions_mut().insert(AuthUser {
                    username: claims.sub,
                    role: claims.role,
                });
                next.run(req).await
            }
            Err(_) => redirect_or_401(&req),
        }
    }
}

fn redirect_or_401(req: &Request) -> Response {
    let accepts_html = req
        .headers()
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/html"))
        .unwrap_or(false);

    if accepts_html {
        Redirect::to("/login").into_response()
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}
