use axum::{
    body::Bytes,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    response::sse::{Event, Sse},
};
use core::convert::Infallible;
use eventsource_stream::Eventsource; // Or EventsourceExt, will fix if compile fails
use futures::StreamExt;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, warn};

use crate::config::Config;
use crate::engine::Engine;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub client: Client,
}

pub async fn chat_completions_handler(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<impl IntoResponse, StatusCode> {
    
    let upstream_req = state.client.post(&state.config.target_endpoint)
        .header("Authorization", format!("Bearer {}", state.config.api_key))
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .body(body);
    
    let upstream_resp = upstream_req.send().await.map_err(|e| {
        error!("Failed to reach upstream: {}", e);
        StatusCode::BAD_GATEWAY
    })?;

    if !upstream_resp.status().is_success() {
        error!("Upstream returned error: {}", upstream_resp.status());
        return Err(StatusCode::BAD_GATEWAY);
    }

    let stream = upstream_resp.bytes_stream().eventsource();
    let mut stream = Box::pin(stream);
    
    let (tx, rx) = mpsc::channel(32);
    let config = state.config.clone();
    
    tokio::spawn(async move {
        let mut engine = match Engine::new(&config.rule) {
            Ok(e) => e,
            Err(e) => {
                error!("Invalid regex rule: {}", e);
                return;
            }
        };

        while let Some(event_res) = stream.next().await {
            match event_res {
                Ok(event) => {
                    let data = event.data;
                    if data == "[DONE]" {
                        let _ = tx.send(Ok::<_, Infallible>(Event::default().data("[DONE]"))).await;
                        break;
                    }
                    
                    // Parse JSON to extract content delta
                    let mut token_str = String::new();
                    if let Ok(json) = serde_json::from_str::<Value>(&data) {
                        if let Some(choices) = json.get("choices").and_then(|c| c.as_array()) {
                            if let Some(choice) = choices.get(0) {
                                if let Some(delta) = choice.get("delta") {
                                    if let Some(content) = delta.get("content").and_then(|c| c.as_str()) {
                                        token_str = content.to_string();
                                    }
                                }
                            }
                        }
                    }

                    if !token_str.is_empty() {
                        if !engine.check_token(&token_str) {
                            warn!("🚨 STREAM KILLED: Token violated constraint. Token: {:?}", token_str);
                            break; // Terminate stream instantly
                        }
                    }
                    
                    // Forward the event downstream
                    if tx.send(Ok::<_, Infallible>(Event::default().data(data))).await.is_err() {
                        // Client disconnected
                        break;
                    }
                }
                Err(e) => {
                    error!("Error reading SSE from upstream: {}", e);
                    break;
                }
            }
        }
    });

    Ok(Sse::new(ReceiverStream::new(rx)))
}
