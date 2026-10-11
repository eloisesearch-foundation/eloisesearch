//! Eloisesearch HTTP/JSON server.
//!
//! Exposed as a library so integration tests can drive the router directly,
//! without binding a socket. The `eloisesearch` binary lives in `main.rs`.

pub mod config;
pub mod error;
pub mod routes;

use std::io;
use std::path::PathBuf;

use eloisesearch_core::DataDir;

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

/// Opens the data directory and serves HTTP until a shutdown signal arrives.
///
/// The data directory stays open (and locked) until the server stops, so a
/// second process pointed at the same directory fails at startup.
///
/// # Errors
///
/// Fails if the data directory cannot be opened (already in use, unsupported
/// format, unreadable metadata) or the address cannot be bound.
pub async fn run(config: Config) -> io::Result<()> {
    let data_dir = open_data_dir(config.data_dir.clone()).await?;
    let listener = TcpListener::bind(config.listen).await?;
    tracing::info!(
        version = eloisesearch_core::VERSION,
        node_id = %data_dir.node().node_id,
        addr = %listener.local_addr()?,
        data_dir = %data_dir.root().display(),
        "eloisesearch listening"
    );

    axum::serve(listener, routes::router(config.request_timeout))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    drop(data_dir);
    tracing::info!("eloisesearch stopped");
    Ok(())
}

/// The core is blocking (file I/O and fsync), so it runs off the async workers.
async fn open_data_dir(path: PathBuf) -> io::Result<DataDir> {
    tokio::task::spawn_blocking(move || DataDir::open(path))
        .await
        .map_err(io::Error::other)?
        .map_err(io::Error::other)
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
