use std::process::ExitCode;

use leaflet::{AppState, config::Config, router};
use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> ExitCode {
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("leaflet: invalid configuration: {err}");
            return ExitCode::FAILURE;
        }
    };

    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(&config.log))
        .init();

    if let Err(err) = std::fs::create_dir_all(config.blobs_dir()) {
        tracing::error!(dir = %config.blobs_dir().display(), %err, "cannot create blob directory");
        return ExitCode::FAILURE;
    }

    let listener = match TcpListener::bind(config.bind).await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::error!(bind = %config.bind, %err, "cannot bind");
            return ExitCode::FAILURE;
        }
    };

    tracing::info!(addr = %config.bind, version = env!("CARGO_PKG_VERSION"), "leaflet listening");

    let app = router(AppState { config });
    if let Err(err) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        tracing::error!(%err, "server error");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}
