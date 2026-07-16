use crate::{Config, PromptMode};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use jsonptr::Pointer;
use log::{debug, error, info};
use minijinja::{Environment, context};
use reqwest::Client;
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};
use tokio::signal;
use tokio::sync::RwLock;
use tokio::sync::watch::Receiver;

#[derive(Clone)]
struct AppState {
    config: Arc<RwLock<Config>>,
    client: Client,
    config_path: PathBuf,
}

/// Start Axum server
pub async fn serve(mut rx: Receiver<Config>, config_path: PathBuf) {
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
    let client = Client::new();
    let state = AppState {
        config,
        client,
        config_path,
    };

    // Get data from config
    let cfg = state.config.read().await;
    let addr = format!("{}:{}", cfg.get_host(), cfg.get_port());
    let api_key = cfg.get_api_key();
    let reload = cfg.is_reload_enabled();
    drop(cfg);

    // Create app and add routes
    let app = Router::new()
        .route("/v1/chat/completions", post(chat_completions))
        .route("/v1/models", get(list_models))
        .with_state(state);

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

// Chat completion api
async fn chat_completions(
    State(state): State<AppState>,
    _headers: HeaderMap,
    Json(mut body): Json<Value>,
) -> Response {
    // Get data
    let model = body.get("model").map_or("", |x| x.as_str().unwrap_or(""));
    let data = state.config.read().await.get_data(model);
    if let Err(e) = data {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": format!("Error extracting model data: {e}") })),
        )
            .into_response();
    }
    let data = data.unwrap();

    // Set model
    body["model"] = json!(data.model_name);

    // Set system prompt
    if let Some(system_prompt) = data.system_prompt {
        if let Some(messages) = body.get_mut("messages").and_then(|v| v.as_array_mut()) {
            if let Some(message) = messages.get(0) {
                // Load template
                let mut env = Environment::new();
                let mut template_path = state.config_path.clone();
                template_path.push("templates");
                env.set_loader(minijinja::path_loader(template_path));
                if let Some(template) = env.get_template(system_prompt.to_str().unwrap()).ok() {
                    // Get requested system_prompt
                    let req_message = message
                        .get("content")
                        .and_then(|v| Some(String::from(v.as_str().unwrap_or(""))))
                        .unwrap_or(String::from(""));

                    // Render new system prompt
                    let mut system_prompt_render = template
                        .render(context!(
                                system_prompt => if data.system_prompt_mode == PromptMode::Combine {
                                    req_message.clone()
                                } else {
                                    String::from("")
                                }
                        ))
                        .unwrap_or(String::from(""));

                    // Add requested system prompt to the end if missing in template
                    let vars = template.undeclared_variables(false);
                    if data.system_prompt_mode == PromptMode::Combine
                        && !vars.contains("system_prompt")
                    {
                        system_prompt_render.push_str(&format!("\n{}", &req_message));
                    }

                    // Remove default system prompt
                    if message
                        .get("role")
                        .and_then(|v| Some(v == "system"))
                        .unwrap_or(false)
                    {
                        if data.system_prompt_mode != PromptMode::Fallback {
                            messages.remove(0);
                        }
                    }

                    // Add new system_prompt
                    messages.insert(
                        0,
                        json!({
                            "role": "system",
                            "content": system_prompt_render
                        }),
                    );
                }
            }
        }
    }

    // Add extra body from config
    for extra in &data.extra_body {
        if let Some(pointer) = Pointer::parse(&extra.pointer).ok() {
            pointer.assign(&mut body, extra.value.clone()).unwrap();
        } else {
            error!("Skipping invalid pointer {}", extra.pointer);
        }
    }

    // Build request
    let mut req = state
        .client
        .post(format!("{}/chat/completions", data.api_base))
        .bearer_auth(data.api_key)
        .json(&body);

    // Add extra headers
    for (k, v) in &data.extra_headers {
        req = req.header(k, v);
    }

    debug!("Request will be sent: {:?}\nContent: {:?}", req, body);

    // Get streaming parameters
    // let stream = body
    //     .get("stream")
    //     .and_then(|v| v.as_bool())
    //     .unwrap_or(false);

    let openai_response = match req.send().await {
        Ok(res) => res,
        Err(err) => {
            tracing::error!(%err, "OpenAI request failed");

            return (
                StatusCode::BAD_GATEWAY,
                format!("OpenAI request failed: {err}"),
            )
                .into_response();
        }
    };

    // Create a response builder with the upstream status code
    let mut response_builder = Response::builder().status(openai_response.status());

    // Forward the headers (crucial for "content-type: text/event-stream")
    if let Some(headers_mut) = response_builder.headers_mut() {
        *headers_mut = openai_response.headers().clone();
    }

    // Convert reqwest's ByteStream into axum's Body
    let stream = openai_response.bytes_stream();
    let body = axum::body::Body::from_stream(stream);

    // Return the finalized response
    match response_builder.body(body) {
        Ok(response) => response,
        Err(err) => {
            tracing::error!(%err, "Failed to build response body");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// List models endpoint
async fn list_models(State(state): State<AppState>) -> Response {
    let cfg = state.config.read().await;
    let names = cfg.list_models();
    let data: Vec<Value> = names
        .into_iter()
        .map(|id| {
            json!({
                "id": id,
                "object": "model",
                "created": 0,
                "owned_by": "user"
            })
        })
        .collect();
    Json(json!({
        "object": "list",
        "data": data
    }))
    .into_response()
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
