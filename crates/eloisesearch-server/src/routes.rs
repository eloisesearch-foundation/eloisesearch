//! HTTP routes. Handlers stay thin: parse, call the core, map the result.

use std::time::Duration;

use axum::http::{Method, StatusCode, Uri};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

use crate::error::ApiError;

/// Builds the application router with its middleware stack.
pub fn router(request_timeout: Duration) -> Router {
    Router::new()
        .route("/health", get(health))
        .fallback(not_found)
        .method_not_allowed_fallback(method_not_allowed)
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            request_timeout,
        ))
        .layer(TraceLayer::new_for_http())
}

/// Liveness response.
#[derive(Debug, Serialize)]
pub struct Health {
    status: &'static str,
    version: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health {
        status: "ok",
        version: eloisesearch_core::VERSION,
    })
}

async fn not_found(method: Method, uri: Uri) -> ApiError {
    ApiError::not_found(format!("no route for {method} {}", uri.path()))
}

async fn method_not_allowed(method: Method, uri: Uri) -> ApiError {
    ApiError::method_not_allowed(format!("{method} is not allowed on {}", uri.path()))
}
