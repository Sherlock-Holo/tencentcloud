//! tencentcloud api library
//!
//! this crate provides a generic [`Client`] and [`api::Api`]

pub use self::client::{Auth, Client};
pub use self::error::Error;

pub mod api;
#[cfg(any(feature = "async-net-native-tls", feature = "async-net-rustls-tls"))]
mod async_net_compat;
pub mod client;
pub mod error;
mod http_client;
mod tc3_hmac;
#[cfg(feature = "tokio-native-tls")]
mod tokio_native_tls_compat;
