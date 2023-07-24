//! error types

use std::error;

use serde::Deserialize;
use thiserror::Error;

/// the error
#[derive(Debug, Error)]
pub enum Error {
    /// tencentcloud api error
    #[error("error: {err}, request id: {request_id}")]
    Api { err: ApiError, request_id: String },

    /// http error
    #[error(transparent)]
    Http(#[from] hyper::Error),

    /// other error
    #[error(transparent)]
    Other(Box<dyn error::Error + Send + Sync + 'static>),

    /// json marshal/unmarshal error
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// tencentcloud api error
///
/// the code and message are returned by tencentcloud api server
#[derive(Debug, Deserialize, Error)]
#[error("code: {code}, message: {message}")]
#[non_exhaustive]
pub struct ApiError {
    /// the error code
    #[serde(rename = "Code")]
    pub code: String,

    /// the error message
    #[serde(rename = "Message")]
    pub message: String,
}
