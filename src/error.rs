use std::error;

use serde::Deserialize;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("error: {err}, request id: {request_id}")]
    Api { err: ApiError, request_id: String },

    #[error(transparent)]
    Http(#[from] reqwest::Error),

    #[error(transparent)]
    Other(Box<dyn error::Error + Send + Sync + 'static>),

    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize, Error)]
#[error("code: {code}, message: {message}")]
#[non_exhaustive]
pub struct ApiError {
    #[serde(rename = "Code")]
    pub code: String,
    #[serde(rename = "Message")]
    pub message: String,
}
