//! Axum server setup and routing.

use axum::{
    extract::{Path, State, WebSocketUpgrade},
    http::StatusCode,
    middleware as axum_mw,
    response::{Html, IntoResponse, Redirect},
    routing::{get, post},
    Extension, Json, Router,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tower_http::services::ServeDir;

use crate::auth::middleware::{require_auth, AuthMode, AuthState, AuthUser};
use crate::auth::{JwtManager, UserStore};
use crate::llm::client::XaiClient;

use super::static_files::{FAVICON_SVG, INDEX_HTML, LOGIN_HTML};
use super::websocket::handle_socket;

pub struct AppState {
    pub data_dir: PathBuf,
    pub xai_client: Arc<XaiClient>,
    pub auth_mode: AuthMode,
    pub user_store: Arc<UserStore>,
    pub jwt_manager: Arc<JwtManager>,
}

#[derive(Deserialize)]
struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    token: String,
    username: String,
    role: String,
}

#[derive(Serialize)]
struct MeResponse {
    username: String,
    role: String,
}

pub async fn run_server(
    port: u16,
    bind_address: &str,
    data_dir: PathBuf,
    require_auth_flag: bool,
) -> anyhow::Result<()> {
    let api_key = std::env::var("XAI_API_KEY").map_err(|_| {
        anyhow::anyhow!(
            "XAI_API_KEY not set. Create a .env file with XAI_API_KEY=your_key"
        )
    })?;

    let model = std::env::var("XAI_MODEL").unwrap_or_else(|_| "grok-4-1-fast-reasoning".to_string());
    let xai_client = Arc::new(XaiClient::new(api_key, model));

    let user_store = Arc::new(UserStore::new(&data_dir));
    let jwt_manager = Arc::new(JwtManager::new(&data_dir)?);

    let auth_mode = if require_auth_flag || user_store.has_users() {
        AuthMode::Enabled
    } else {
        AuthMode::Disabled
    };

    let app_state = Arc::new(AppState {
        data_dir: data_dir.clone(),
        xai_client,
        auth_mode: auth_mode.clone(),
        user_store: user_store.clone(),
        jwt_manager: jwt_manager.clone(),
    });

    let auth_state = Arc::new(AuthState {
        auth_mode: auth_mode.clone(),
        jwt_manager: jwt_manager.clone(),
    });

    // Static file directories
    let static_dir = std::env::current_dir()?.join("static");

    // Public routes
    let public_routes = Router::new()
        .route("/login", get(login_page))
        .route("/api/auth/login", post(login_handler))
        .route("/health", get(|| async { "ok" }))
        .route("/favicon.svg", get(favicon));

    // Protected routes
    let protected_routes = Router::new()
        .route("/", get(index_page))
        .route("/adventure/{id}", get(index_page))
        .route("/api/auth/me", get(me_handler))
        .route("/ws", get(ws_handler))
        .layer(axum_mw::from_fn_with_state(
            auth_state.clone(),
            require_auth,
        ));

    let app = Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .nest_service("/css", ServeDir::new(static_dir.join("css")))
        .nest_service("/js", ServeDir::new(static_dir.join("js")))
        .nest_service("/assets", ServeDir::new(static_dir.join("assets")))
        .with_state(app_state);

    let addr = format!("{}:{}", bind_address, port);
    eprintln!("RuneQuest server starting on http://{}", addr);
    eprintln!("Auth mode: {:?}", auth_mode);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn login_page() -> Html<&'static str> {
    Html(LOGIN_HTML)
}

async fn index_page() -> Html<&'static str> {
    Html(INDEX_HTML)
}

async fn favicon() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "image/svg+xml")],
        FAVICON_SVG,
    )
}

async fn login_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoginRequest>,
) -> impl IntoResponse {
    match state.user_store.authenticate(&req.username, &req.password) {
        Ok(user) => {
            let role = user.role.to_string();
            match state.jwt_manager.create_token(&user.username, &role) {
                Ok(token) => Json(LoginResponse {
                    token,
                    username: user.username,
                    role,
                })
                .into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Err(_) => StatusCode::UNAUTHORIZED.into_response(),
    }
}

async fn me_handler(Extension(user): Extension<AuthUser>) -> Json<MeResponse> {
    Json(MeResponse {
        username: user.username,
        role: user.role,
    })
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(user): Extension<AuthUser>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| {
        handle_socket(
            socket,
            user,
            state.xai_client.clone(),
            state.data_dir.clone(),
        )
    })
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install CTRL+C handler");
    eprintln!("\nShutting down...");
}
