use crate::Config;
use axum::{
    Json, Router,
    http::StatusCode,
    routing::{get, post},
};
use log::{info, debug};
use std::sync::Arc;
use tokio::signal;
use tokio::sync::RwLock;
use tokio::sync::watch::Receiver;

#[derive(Clone)]
struct AppState {
    config: Arc<RwLock<Config>>,
}

/// Start Axum server
pub async fn serve(mut rx: Receiver<Config>) {
    // Spawn config update task
    let config = rx.borrow_and_update().clone();
    let config = Arc::new(RwLock::new(config));
    let config_clone = config.clone();
    tokio::spawn(async move {
        let mut rx = rx;
        while rx.changed().await.is_ok() {
            let new_config = rx.borrow_and_update().clone();
            let mut guard = config_clone.write().await;
            debug!("New config: {:?}", &new_config);
            *guard = new_config;
            info!("Config reloaded");
        }
    });

    // Create app state
    let state = AppState { config };

    // Get data from config
    let cfg = state.config.read().await;
    let addr = format!("{}:{}", cfg.get_host(), cfg.get_port());
    let api_key = cfg.get_api_key();
    let reload = cfg.is_reload_enabled();
    drop(cfg);


    // Create app and add routes
    let app = Router::new().with_state(state);

    // `GET /` goes to `root`
    // .route("/", get(root))
    // `POST /users` goes to `create_user`
    // .route("/users", post(create_user));

    // Create listener
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();

    // Log info
    info!("Application is running on: {}", addr);
    if let Some(key) = api_key {
        let masked = if key.len() < 5 {
            "*".repeat(16)
        } else {
            let mut out = String::with_capacity(16);
            out.push_str(&key[..3]);
            out.push_str(&"*".repeat(11));
            out.push_str(&key[key.len() - 2..]);
            out
        };
        info!("API key: {}", masked);
    }
    if reload {
        info!("Config hot-reload is enabled")
    }

    // Start app
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

// Shutdown gracefully
// https://github.com/tokio-rs/axum/blob/main/examples/graceful-shutdown/src/main.rs
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
           eprintln!();
           info!("Ctrl+C pressed. Shutting down...");
        },
        _ = terminate => {
           info!("Termination signal received. Shutting down...");
        },
    }
}
