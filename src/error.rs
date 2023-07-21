use std::error;

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    /// tencentcloud api error
    #[error("error: {err}, request id: {request_id}")]
    Api { err: ApiError, request_id: String },

    /// http error
    #[error(transparent)]
    Http(#[from] reqwest::Error),

    /// other error
    #[error(transparent)]
    Other(Box<dyn error::Error + Send + Sync + 'static>),

    /// json marshal/unmarshal error
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

/// tencentcloud api error
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
