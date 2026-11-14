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

    let pool = match leaflet::db::open(&config).await {
        Ok(pool) => pool,
        Err(err) => {
            tracing::error!(%err, "cannot open database");
            return ExitCode::FAILURE;
        }
    };

    let storage = match leaflet::storage::Storage::new(config.blobs_dir()) {
        Ok(storage) => storage,
        Err(err) => {
            tracing::error!(%err, "cannot prepare blob storage");
            return ExitCode::FAILURE;
        }
    };

    let signing = match leaflet::auth::tokens::SigningKey::load_or_create(
        &config.data_dir,
        std::env::var("LEAFLET_SECRET").ok().as_deref(),
    ) {
        Ok((signing, source)) => {
            tracing::info!(?source, "access token signing key ready");
            signing
        }
        Err(err) => {
            tracing::error!(%err, "cannot prepare the signing key");
            return ExitCode::FAILURE;
        }
    };

    let listener = match TcpListener::bind(config.bind).await {
        Ok(listener) => listener,
        Err(err) => {
            tracing::error!(bind = %config.bind, %err, "cannot bind");
            return ExitCode::FAILURE;
        }
    };

    tracing::info!(addr = %config.bind, version = env!("CARGO_PKG_VERSION"), "leaflet listening");

    let assets = leaflet::assets::Assets::new(&config.web_dir);
    if assets.resolved().is_none() {
        tracing::warn!(dir = %config.web_dir.display(), "no web export found; serving api only");
    }

    let app = router(AppState {
        config,
        pool,
        storage,
        assets,
        signing,
    });
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
