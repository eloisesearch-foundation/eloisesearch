//! HTTP-level tests that drive the router in-process (no socket).

use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

fn app() -> Router {
    eloisesearch_server::routes::router(Duration::from_secs(5))
}

async fn call(method: &str, uri: &str) -> (StatusCode, Value) {
    let response = app()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    assert_eq!(
        response.headers()["content-type"],
        "application/json",
        "every response is JSON"
    );
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn health_reports_ok_and_version() {
    let (status, body) = call("GET", "/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        body,
        json!({ "status": "ok", "version": eloisesearch_core::VERSION })
    );
}

#[tokio::test]
async fn unknown_route_uses_error_model() {
    let (status, body) = call("GET", "/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        body,
        json!({
            "error": { "type": "not_found", "reason": "no route for GET /nope" },
            "status": 404
        })
    );
}

#[tokio::test]
async fn wrong_method_uses_error_model() {
    let (status, body) = call("POST", "/health").await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(body["error"]["type"], "method_not_allowed");
    assert_eq!(body["status"], 405);
}
