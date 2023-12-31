//! tencentcloud api client

use std::fmt::{Debug, Formatter};

use http_body::Limited;
use hyper::{body, Body, Method, Request, StatusCode};
use serde::Deserialize;
use time::OffsetDateTime;
use tracing::{instrument, trace};

use crate::api::Api;
use crate::error::{ApiError, Error};
use crate::http_client::{new_http_client, HttpClient};
use crate::tc3_hmac;

/// tencentcloud api client
///
/// the [`Client`] can be used to send request to tencentcloud and get the response
#[derive(Debug, Clone)]
pub struct Client {
    region: String,
    http_client: HttpClient,
    auth: Auth,
    response_size_limit: Option<usize>,
}

impl Client {
    /// create an api client
    ///
    /// `response_size_limit` is used to limit the http response size, if `response_size_limit` is
    /// set and response body size is bigger then `response_size_limit`, will return error
    pub fn new(region: String, auth: Auth, response_size_limit: impl Into<Option<usize>>) -> Self {
        Self {
            region,
            http_client: new_http_client(),
            auth,
            response_size_limit: response_size_limit.into(),
        }
    }

    /// send api request, get the api response and request id
    ///
    /// the `request` and `response` types are defined by the `A`: [`Api`]
    #[instrument(level = "trace", err)]
    pub async fn send<A: Api>(&self, request: &A::Request) -> Result<(A::Response, String), Error> {
        let payload = serde_json::to_vec(request)?;

        trace!("marshal request done");

        let request = self.create_request::<A>(payload)?;

        trace!(?request, "create http request done");

        let response = self.http_client.request(request).await?;

        trace!(?response, "get http response done");

        if response.status() != StatusCode::OK {
            return Err(Error::Other(
                format!("status code is not OK: {}", response.status()).into(),
            ));
        }

        let body = response.into_body();
        let body = match self.response_size_limit {
            None => body::to_bytes(body).await?,

            Some(limit) => body::to_bytes(Limited::new(body, limit))
                .await
                .map_err(|err| Error::Other(err))?,
        };

        trace!("read http body done");

        let response = serde_json::from_slice::<Response<A::Response>>(&body)?;

        trace!(?response, "unmarshal response done");

        if let Some(err) = response.response.error {
            return Err(Error::Api {
                err,
                request_id: response.response.request_id,
            });
        }

        match response.response.response {
            None => Err(Error::Other("miss response".into())),
            Some(resp) => Ok((resp, response.response.request_id)),
        }
    }

    #[instrument(level = "trace", err)]
    fn create_request<A: Api>(&self, payload: Vec<u8>) -> Result<Request<Body>, Error> {
        let now = OffsetDateTime::now_utc();
        let authorization = tc3_hmac::calculate_authorization(
            &self.auth.secret_id,
            &self.auth.secret_key,
            A::SERVICE,
            A::HOST,
            &payload,
            &now,
        )
        .map_err(Error::Other)?;

        let request = Request::builder()
            .uri("https://tmt.tencentcloudapi.com")
            .method(Method::POST)
            .header("Authorization", authorization)
            .header("Content-Type", "application/json; charset=utf-8")
            .header("X-TC-Action", A::ACTION)
            .header("X-TC-Timestamp", now.unix_timestamp())
            .header("X-TC-Version", A::VERSION)
            .header("X-TC-Region", &self.region)
            .body(Body::from(payload))
            .map_err(|err| Error::Other(err.into()))?;

        Ok(request)
    }
}

/// tencentcloud api auth
///
/// currently only support `secret_key` and `secret_id`
#[derive(Clone)]
pub struct Auth {
    secret_key: String,
    secret_id: String,
}

impl Auth {
    /// create tencentcloud api auth by `secret_key` and `secret_id`
    ///
    /// you can get the `secret_key` and `secret_id` from tencentcloud web console
    pub fn new(secret_key: String, secret_id: String) -> Self {
        Self {
            secret_key,
            secret_id,
        }
    }
}

impl Debug for Auth {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Auth")
            .field("secret_id", &self.secret_id)
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Deserialize)]
struct Response<T> {
    #[serde(rename = "Response")]
    response: ResponseDetail<T>,
}

#[derive(Debug, Deserialize)]
struct ResponseDetail<T> {
    #[serde(rename = "RequestId")]
    request_id: String,

    #[serde(flatten)]
    response: Option<T>,

    #[serde(rename = "Error")]
    error: Option<ApiError>,
}
