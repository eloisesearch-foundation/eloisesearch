//! Eloisesearch HTTP/JSON server.
//!
//! Exposed as a library so integration tests can drive the router directly,
//! without binding a socket. The `eloisesearch` binary lives in `main.rs`.

pub mod config;
pub mod error;
pub mod routes;

use std::io;

use tokio::net::TcpListener;
use tracing_subscriber::EnvFilter;

pub use config::{Config, LogFormat};

/// Initializes structured logging. Verbosity comes from `RUST_LOG`
/// (default: `info` for Eloisesearch, `warn` for dependencies).
pub fn init_tracing(format: LogFormat) {
    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new("warn,eloisesearch=info,eloisesearch_server=info,eloisesearch_core=info,tower_http=info")
    });
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    match format {
        LogFormat::Text => builder.init(),
        LogFormat::Json => builder.json().flatten_event(true).init(),
    }
}

/// Prepares the data directory and serves HTTP until a shutdown signal arrives.
///
/// # Errors
///
/// Fails if the data directory cannot be created or the address cannot be bound.
pub async fn run(config: Config) -> io::Result<()> {
    tokio::fs::create_dir_all(&config.data_dir).await?;
    let listener = TcpListener::bind(config.listen).await?;
    tracing::info!(
        version = eloisesearch_core::VERSION,
        addr = %listener.local_addr()?,
        data_dir = %config.data_dir.display(),
        "eloisesearch listening"
    );

    axum::serve(listener, routes::router(config.request_timeout))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("eloisesearch stopped");
    Ok(())
}

async fn shutdown_signal() {
    let ctrl_c = async {
        if let Err(err) = tokio::signal::ctrl_c().await {
            tracing::error!(%err, "failed to listen for Ctrl+C");
        }
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(err) => {
                tracing::error!(%err, "failed to listen for SIGTERM");
                std::future::pending::<()>().await;
            }
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("shutdown signal received, draining connections");
}
