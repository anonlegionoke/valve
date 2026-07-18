mod cli;
mod config;
mod engine;
mod proxy;

use axum::{
    routing::post,
    Router,
};
use clap::Parser;
use std::sync::Arc;
use tracing::{info, error};

use crate::cli::Cli;
use crate::config::Config;
use crate::proxy::{chat_completions_handler, AppState};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env().add_directive("valve=info".parse().unwrap()))
        .init();

    let cli = Cli::parse();

    let config = match Config::load(&cli.config) {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to load configuration from {}: {}", cli.config, e);
            std::process::exit(1);
        }
    };

    let client = reqwest::Client::new();
    let app_state = AppState {
        config: Arc::new(config),
        client,
    };

    let app = Router::new()
        .route("/v1/chat/completions", post(chat_completions_handler))
        .with_state(app_state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", cli.port)).await.unwrap();
    info!("Starting valve on port {}", cli.port);

    axum::serve(listener, app).await.unwrap();
}
