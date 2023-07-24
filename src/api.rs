//! the [`Api`] trait allows user define the tencentcloud an api
//!
//! ## Examples:
//!
//! ```rust
//! use serde::{Deserialize, Serialize};
//! use tencentcloud::api::Api;
//!
//! #[derive(Debug, Copy, Clone)]
//! pub struct TextTranslate;
//!
//! #[derive(Debug, Clone, Serialize)]
//! pub struct TextTranslateRequest {
//!     #[serde(rename = "SourceText")]
//!     pub source_text: String,
//!     #[serde(rename = "Source")]
//!     pub source: String,
//!     #[serde(rename = "Target")]
//!     pub target: String,
//!     #[serde(rename = "ProjectId")]
//!     pub project_id: i64,
//! }
//!
//! #[derive(Debug, Clone, Deserialize)]
//! pub struct TextTranslateResponse {
//!     #[serde(rename = "Source")]
//!     pub source: String,
//!     #[serde(rename = "Target")]
//!     pub target: String,
//!     #[serde(rename = "TargetText")]
//!     pub target_text: String,
//! }
//!
//! impl Api for TextTranslate {
//!     type Request = TextTranslateRequest;
//!     type Response = TextTranslateResponse;
//!     const VERSION: &'static str = "2018-03-21";
//!     const ACTION: &'static str = "TextTranslate";
//!     const SERVICE: &'static str = "tmt";
//!     const HOST: &'static str = "tmt.tencentcloudapi.com";
//! }
//! ```

use std::fmt::Debug;

use serde::{Deserialize, Serialize};

/// tencentcloud api
///
/// the [`Api`] define an api request type, response type and other info
pub trait Api {
    /// the api request type
    type Request: Serialize + Debug;

    /// the api response type
    type Response: for<'a> Deserialize<'a> + Debug;

    /// the api version, format is `2018-3-21`
    const VERSION: &'static str;

    /// the api action, for example: the tmt text translate is `TextTranslate`
    const ACTION: &'static str;

    /// the api service, for example: the tmt text translate is `tmt`
    const SERVICE: &'static str;

    /// the api host, for example: the tmt text translate is `tmt.tencentcloudapi.com`
    const HOST: &'static str;
}
