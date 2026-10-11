//! Runtime configuration, read from `ELOISESEARCH_*` environment variables.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;

use thiserror::Error;

/// Log output format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Human-readable, for local development.
    Text,
    /// One JSON object per line, for containers and log pipelines.
    Json,
}

/// Server configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Address the HTTP server listens on.
    pub listen: SocketAddr,
    /// Directory holding indexes and recovery data. Must be persistent storage.
    pub data_dir: PathBuf,
    /// Log output format.
    pub log_format: LogFormat,
    /// Maximum time a request may take before it is aborted.
    pub request_timeout: Duration,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 7700),
            data_dir: PathBuf::from("./data"),
            log_format: LogFormat::Text,
            request_timeout: Duration::from_secs(30),
        }
    }
}

/// Invalid configuration value.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("invalid value {value:?} for {var}: {reason}")]
pub struct ConfigError {
    var: &'static str,
    value: String,
    reason: &'static str,
}

impl Config {
    /// Reads the configuration from the process environment.
    ///
    /// | Variable | Default |
    /// | --- | --- |
    /// | `ELOISESEARCH_HOST` | `127.0.0.1` |
    /// | `ELOISESEARCH_PORT` | `7700` |
    /// | `ELOISESEARCH_DATA_DIR` | `./data` |
    /// | `ELOISESEARCH_LOG_FORMAT` | `text` (`text` or `json`) |
    /// | `ELOISESEARCH_REQUEST_TIMEOUT_SECS` | `30` |
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigError`] naming the first invalid variable.
    pub fn from_env() -> Result<Self, ConfigError> {
        Self::from_lookup(|key| std::env::var(key).ok())
    }

    /// Builds the configuration from an arbitrary key lookup (used by tests).
    ///
    /// # Errors
    ///
    /// Returns a [`ConfigError`] naming the first invalid variable.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Result<Self, ConfigError> {
        let defaults = Self::default();

        let host = match lookup("ELOISESEARCH_HOST") {
            Some(v) => v.parse::<IpAddr>().map_err(|_| ConfigError {
                var: "ELOISESEARCH_HOST",
                value: v,
                reason: "expected an IP address such as 127.0.0.1 or 0.0.0.0",
            })?,
            None => defaults.listen.ip(),
        };
        let port = match lookup("ELOISESEARCH_PORT") {
            Some(v) => v.parse::<u16>().map_err(|_| ConfigError {
                var: "ELOISESEARCH_PORT",
                value: v,
                reason: "expected a port number between 0 and 65535",
            })?,
            None => defaults.listen.port(),
        };
        let data_dir = lookup("ELOISESEARCH_DATA_DIR").map_or(defaults.data_dir, PathBuf::from);
        let log_format = match lookup("ELOISESEARCH_LOG_FORMAT").as_deref() {
            None | Some("text") => LogFormat::Text,
            Some("json") => LogFormat::Json,
            Some(other) => {
                return Err(ConfigError {
                    var: "ELOISESEARCH_LOG_FORMAT",
                    value: other.to_owned(),
                    reason: "expected \"text\" or \"json\"",
                });
            }
        };
        let request_timeout = match lookup("ELOISESEARCH_REQUEST_TIMEOUT_SECS") {
            Some(v) => match v.parse::<u64>() {
                Ok(secs) if secs > 0 => Duration::from_secs(secs),
                _ => {
                    return Err(ConfigError {
                        var: "ELOISESEARCH_REQUEST_TIMEOUT_SECS",
                        value: v,
                        reason: "expected a positive number of seconds",
                    });
                }
            },
            None => defaults.request_timeout,
        };

        Ok(Self {
            listen: SocketAddr::new(host, port),
            data_dir,
            log_format,
            request_timeout,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;

    fn from(pairs: &[(&str, &str)]) -> Result<Config, ConfigError> {
        let map: HashMap<String, String> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        Config::from_lookup(|k| map.get(k).cloned())
    }

    #[test]
    fn defaults_when_unset() {
        assert_eq!(from(&[]).unwrap(), Config::default());
    }

    #[test]
    fn reads_all_variables() {
        let config = from(&[
            ("ELOISESEARCH_HOST", "0.0.0.0"),
            ("ELOISESEARCH_PORT", "8080"),
            ("ELOISESEARCH_DATA_DIR", "/var/lib/eloisesearch"),
            ("ELOISESEARCH_LOG_FORMAT", "json"),
            ("ELOISESEARCH_REQUEST_TIMEOUT_SECS", "5"),
        ])
        .unwrap();
        assert_eq!(config.listen, "0.0.0.0:8080".parse().unwrap());
        assert_eq!(config.data_dir, PathBuf::from("/var/lib/eloisesearch"));
        assert_eq!(config.log_format, LogFormat::Json);
        assert_eq!(config.request_timeout, Duration::from_secs(5));
    }

    #[test]
    fn rejects_invalid_values() {
        for (var, value) in [
            ("ELOISESEARCH_HOST", "localhost"),
            ("ELOISESEARCH_PORT", "70000"),
            ("ELOISESEARCH_LOG_FORMAT", "yaml"),
            ("ELOISESEARCH_REQUEST_TIMEOUT_SECS", "0"),
        ] {
            let err = from(&[(var, value)]).unwrap_err();
            assert_eq!(err.var, var);
        }
    }
}
