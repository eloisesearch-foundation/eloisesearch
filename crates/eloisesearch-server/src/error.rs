//! The HTTP error model.
//!
//! Every error response has the same shape, the one used by the compatible
//! search API (see `docs/api-contract.md`):
//!
//! ```json
//! {
//!   "error": {
//!     "root_cause": [{ "type": "not_found", "reason": "no route for GET /nope" }],
//!     "type": "not_found",
//!     "reason": "no route for GET /nope"
//!   },
//!   "status": 404
//! }
//! ```
//!
//! `type` is a stable, machine-readable identifier; `reason` is for humans and
//! may change between versions. `root_cause` lists the underlying causes; for
//! errors without a deeper cause it repeats the error itself.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// An error returned to API clients.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    status: StatusCode,
    kind: &'static str,
    reason: String,
}

impl ApiError {
    /// Builds an error with an explicit status and type.
    pub fn new(status: StatusCode, kind: &'static str, reason: impl Into<String>) -> Self {
        Self {
            status,
            kind,
            reason: reason.into(),
        }
    }

    /// 400 — the request is syntactically or semantically invalid.
    pub fn bad_request(kind: &'static str, reason: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, kind, reason)
    }

    /// 404 — the route or resource does not exist.
    pub fn not_found(reason: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", reason)
    }

    /// 405 — the route exists but not for this HTTP method.
    pub fn method_not_allowed(reason: impl Into<String>) -> Self {
        Self::new(StatusCode::METHOD_NOT_ALLOWED, "method_not_allowed", reason)
    }

    /// 408 — the request did not finish within the configured timeout.
    pub fn request_timeout(reason: impl Into<String>) -> Self {
        Self::new(StatusCode::REQUEST_TIMEOUT, "request_timeout", reason)
    }

    /// 500 — unexpected failure. The reason is logged, not exposed.
    pub fn internal(reason: impl std::fmt::Display) -> Self {
        tracing::error!(%reason, "internal error");
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal_error",
            "an internal error occurred",
        )
    }

    /// HTTP status of this error.
    pub fn status(&self) -> StatusCode {
        self.status
    }
}

#[derive(Serialize)]
struct Body<'a> {
    error: Detail<'a>,
    status: u16,
}

#[derive(Serialize)]
struct Detail<'a> {
    root_cause: [Cause<'a>; 1],
    #[serde(rename = "type")]
    kind: &'a str,
    reason: &'a str,
}

#[derive(Serialize)]
struct Cause<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    reason: &'a str,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Body {
            error: Detail {
                root_cause: [Cause {
                    kind: self.kind,
                    reason: &self.reason,
                }],
                kind: self.kind,
                reason: &self.reason,
            },
            status: self.status.as_u16(),
        };
        (self.status, Json(body)).into_response()
    }
}

impl From<eloisesearch_core::Error> for ApiError {
    fn from(err: eloisesearch_core::Error) -> Self {
        match err {
            eloisesearch_core::Error::InvalidIndexName { .. } => {
                Self::bad_request("invalid_index_name_exception", err.to_string())
            }
            other => Self::internal(other),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_index_name_maps_to_400_with_compatible_type() {
        let err = eloisesearch_core::IndexName::new("Bad Name").unwrap_err();
        let api = ApiError::from(err);
        assert_eq!(api.status(), StatusCode::BAD_REQUEST);
        assert_eq!(api.kind, "invalid_index_name_exception");
    }
}
