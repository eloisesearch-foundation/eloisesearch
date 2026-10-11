//! HTTP-level tests that drive the router in-process (no socket).

use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::routing::get;
use eloisesearch_server::routes::{router, with_middleware};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use tower::ServiceExt;

async fn send(app: Router, method: &str, uri: &str) -> (StatusCode, Value) {
    let response = app
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
        response
            .headers()
            .get("content-type")
            .map(|v| v.to_str().unwrap()),
        Some("application/json"),
        "every response is JSON (status {status})"
    );
    let bytes = response.into_body().collect().await.unwrap().to_bytes();
    (status, serde_json::from_slice(&bytes).unwrap())
}

async fn call(method: &str, uri: &str) -> (StatusCode, Value) {
    send(router(Duration::from_secs(5)), method, uri).await
}

/// Asserts the shared error envelope and returns its `type`.
fn assert_error_envelope(status: StatusCode, body: &Value) -> String {
    let kind = body["error"]["type"]
        .as_str()
        .expect("error.type")
        .to_owned();
    let reason = body["error"]["reason"].as_str().expect("error.reason");
    assert_eq!(body["status"], status.as_u16());
    assert_eq!(
        body["error"]["root_cause"],
        json!([{ "type": kind, "reason": reason }]),
        "root_cause repeats the error when there is no deeper cause"
    );
    kind
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
async fn unknown_route_uses_error_envelope() {
    let (status, body) = call("GET", "/nope").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        body,
        json!({
            "error": {
                "root_cause": [{ "type": "not_found", "reason": "no route for GET /nope" }],
                "type": "not_found",
                "reason": "no route for GET /nope"
            },
            "status": 404
        })
    );
}

#[tokio::test]
async fn wrong_method_uses_error_envelope() {
    let (status, body) = call("POST", "/health").await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    assert_eq!(assert_error_envelope(status, &body), "method_not_allowed");
}

#[tokio::test]
async fn timeout_uses_error_envelope() {
    let slow = Router::new().route(
        "/slow",
        get(|| async {
            tokio::time::sleep(Duration::from_secs(5)).await;
            "too late"
        }),
    );
    let app = with_middleware(slow, Duration::from_millis(20));

    let (status, body) = send(app, "GET", "/slow").await;
    assert_eq!(status, StatusCode::REQUEST_TIMEOUT);
    assert_eq!(assert_error_envelope(status, &body), "request_timeout");
}

#[tokio::test]
async fn requests_within_the_timeout_are_unaffected() {
    let fast = Router::new().route("/fast", get(|| async { axum::Json(json!({ "ok": true })) }));
    let app = with_middleware(fast, Duration::from_millis(500));

    let (status, body) = send(app, "GET", "/fast").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, json!({ "ok": true }));
}
