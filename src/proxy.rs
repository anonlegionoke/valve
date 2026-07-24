use axum::{
    body::Bytes,
    extract::State,
    http::{StatusCode, HeaderMap},
    response::IntoResponse,
    response::sse::{Event, Sse},
};
use core::convert::Infallible;
use eventsource_stream::Eventsource;
use futures::StreamExt;
use reqwest::Client;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::{error, warn, info};

use crate::config::Config;
use crate::engine::Engine;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub client: Client,
}

pub async fn chat_completions_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, StatusCode> {
    
    // Parse the incoming JSON body to auto-detect the model
    let json_body: Value = serde_json::from_slice(&body).map_err(|e| {
        error!("Failed to parse JSON body: {}", e);
        StatusCode::BAD_REQUEST
    })?;

    let model_name = json_body.get("model").and_then(|m| m.as_str()).unwrap_or("");
    
    // Auto-detect routing logic based on model prefix
    let provider_key = if model_name.starts_with("gpt-") || model_name.starts_with("o1-") {
        "openai"
    } else if model_name.starts_with("gemini-") {
        "gemini"
    } else {
        warn!("Unknown model requested: {}, defaulting to openai", model_name);
        "openai"
    };

    let provider = state.config.providers.get(provider_key).ok_or_else(|| {
        error!("Provider configuration missing for {}", provider_key);
        StatusCode::INTERNAL_SERVER_ERROR
    })?.clone();
    
    // Determine which rule to use
    let rule_key = headers.get("x-valve-rule")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("default");

    let rule_config = match state.config.rules.get(rule_key) {
        Some(r) => r.clone(),
        None => {
            warn!("Requested rule '{}' not found in configuration", rule_key);
            return Err(StatusCode::BAD_REQUEST);
        }
    };
    
    info!("Routing request for model '{}' to provider '{}' using rule '{}'", model_name, provider_key, rule_key);

    let (tx, rx) = mpsc::channel(32);
    let client = state.client.clone();
    
    tokio::spawn(async move {
        let mut engine = match Engine::new(&rule_config) {
            Ok(e) => e,
            Err(e) => {
                error!("Invalid rule config '{:?}': {}", rule_config, e);
                return;
            }
        };

        let mut payload = json_body.clone();
        let mut max_retries = 3;

        loop {
            let upstream_req = client.post(&provider.endpoint)
                .header("Authorization", format!("Bearer {}", provider.api_key))
                .header("Content-Type", "application/json")
                .header("Accept", "text/event-stream")
                .json(&payload);
            
            let upstream_resp = match upstream_req.send().await {
                Ok(resp) => resp,
                Err(e) => {
                    error!("Failed to reach upstream: {}", e);
                    break;
                }
            };

            if !upstream_resp.status().is_success() {
                error!("Upstream returned error: {}", upstream_resp.status());
                break;
            }

            let stream = upstream_resp.bytes_stream().eventsource();
            let mut stream = Box::pin(stream);
            
            let mut stream_completed = false;
            let mut violated = false;
            let mut generated_text = String::new();

            while let Some(event_res) = stream.next().await {
                match event_res {
                    Ok(event) => {
                        let data = event.data;
                        if data == "[DONE]" {
                            stream_completed = true;
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
                            let start = std::time::Instant::now();
                            let is_valid = engine.check_token(&token_str);
                            let elapsed = start.elapsed();
                            
                            info!("Token evaluated in {}µs", elapsed.as_micros());

                            if !is_valid {
                                warn!("🚨 STREAM KILLED: Token violated constraint. Token: {:?}", token_str);
                                engine.pop_token(&token_str); // Undo the bad token
                                violated = true;
                                break; // Terminate inner stream instantly
                            }
                            
                            generated_text.push_str(&token_str);
                        }
                        
                        // Forward the event downstream
                        if tx.send(Ok::<_, Infallible>(Event::default().data(data))).await.is_err() {
                            return; // Client disconnected
                        }
                    }
                    Err(e) => {
                        error!("Error reading SSE from upstream: {}", e);
                        break;
                    }
                }
            }
            
            if stream_completed || !violated {
                let _ = tx.send(Ok::<_, Infallible>(Event::default().data("[DONE]"))).await;
                break;
            }
            
            // If we reached here, the rule was violated. Self-heal and retry!
            max_retries -= 1;
            if max_retries == 0 {
                warn!("Max retries reached. Aborting self-healing.");
                let _ = tx.send(Ok::<_, Infallible>(Event::default().data("[DONE]"))).await;
                break;
            }
            
            info!("Self-Healing triggered! Retrying stream...");
            
            // Append generated text so far, plus a corrective prompt to the payload's messages array.
            if let Some(messages) = payload.get_mut("messages").and_then(|m| m.as_array_mut()) {
                if !generated_text.is_empty() {
                    messages.push(serde_json::json!({
                        "role": "assistant",
                        "content": generated_text
                    }));
                }
                messages.push(serde_json::json!({
                    "role": "user",
                    "content": "You made a formatting error. Continue generating from exactly where you left off and fix the formatting."
                }));
            }
        }
    });

    Ok(Sse::new(ReceiverStream::new(rx)))
}
