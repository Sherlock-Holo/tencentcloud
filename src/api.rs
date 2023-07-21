use std::fmt::Debug;

use serde::{Deserialize, Serialize};

/// tencentcloud api
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
