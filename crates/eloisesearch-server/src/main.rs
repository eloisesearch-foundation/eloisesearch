use std::process::ExitCode;

use eloisesearch_server::{Config, init_tracing, run};

#[tokio::main]
async fn main() -> ExitCode {
    let config = match Config::from_env() {
        Ok(config) => config,
        Err(err) => {
            eprintln!("eloisesearch: {err}");
            return ExitCode::from(2);
        }
    };
    init_tracing(config.log_format);

    match run(config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            tracing::error!(%err, "eloisesearch failed");
            ExitCode::FAILURE
        }
    }
}
