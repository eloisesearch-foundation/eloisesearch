//! HTTP routes. Handlers stay thin: parse, call the core, map the result.

use std::time::Duration;

use axum::error_handling::HandleErrorLayer;
use axum::http::{Method, Uri};
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use tower::timeout::TimeoutLayer;
use tower::timeout::error::Elapsed;
use tower::{BoxError, ServiceBuilder};
use tower_http::trace::TraceLayer;

use crate::error::ApiError;

/// Builds the application router with its middleware stack.
pub fn router(request_timeout: Duration) -> Router {
    with_middleware(Router::new().route("/health", get(health)), request_timeout)
}

/// Applies the fallbacks and middleware shared by every route.
///
/// Public so tests can mount extra routes (for example a deliberately slow
/// one) behind exactly the same stack as production.
pub fn with_middleware(routes: Router, request_timeout: Duration) -> Router {
    routes
        .fallback(not_found)
        .method_not_allowed_fallback(method_not_allowed)
        // The timeout produces an error instead of a response; `HandleErrorLayer`
        // turns it into the standard JSON error body. A response-producing
        // timeout layer would answer 408 with an empty body.
        .layer(
            ServiceBuilder::new()
                .layer(HandleErrorLayer::new(handle_middleware_error))
                .layer(TimeoutLayer::new(request_timeout)),
        )
        .layer(TraceLayer::new_for_http())
}

async fn handle_middleware_error(err: BoxError) -> ApiError {
    if err.is::<Elapsed>() {
        ApiError::request_timeout("request exceeded the configured timeout")
    } else {
        ApiError::internal(err)
    }
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
