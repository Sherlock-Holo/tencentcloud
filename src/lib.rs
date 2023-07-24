//! tencentcloud api library
//!
//! this crate provides a generic [`Client`] and [`api::Api`]

pub use self::client::{Auth, Client};
pub use self::error::Error;

pub mod api;
#[cfg(any(feature = "async-std-native-tls", feature = "async-std-rustls-tls"))]
mod async_std_compat;
pub mod client;
pub mod error;
mod http_client;
mod tc3_hmac;
#[cfg(feature = "tokio-native-tls")]
mod tokio_native_tls_compat;
