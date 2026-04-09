#[cfg(all(
    any(feature = "async-net-rustls-tls", feature = "async-net-native-tls"),
    not(feature = "tokio-rustls-tls"),
    not(feature = "tokio-native-tls")
))]
use crate::async_net_compat::{Connector, HyperExecutor};
use bytes::Bytes;
use http_body_util::Full;
#[cfg(all(
    feature = "tokio-rustls-tls",
    not(feature = "async-net-rustls-tls"),
    not(feature = "async-net-native-tls")
))]
use hyper_rustls::HttpsConnector;
#[cfg(all(
    feature = "tokio-rustls-tls",
    not(feature = "async-net-rustls-tls"),
    not(feature = "async-net-native-tls")
))]
use hyper_util::client::legacy::connect::HttpConnector;
#[cfg(all(
    any(feature = "tokio-rustls-tls", feature = "tokio-native-tls"),
    not(feature = "async-net-rustls-tls"),
    not(feature = "async-net-native-tls")
))]
use hyper_util::rt::TokioExecutor;

#[cfg(all(
    feature = "tokio-rustls-tls",
    not(feature = "async-net-rustls-tls"),
    not(feature = "async-net-native-tls")
))]
pub type HttpClient =
    hyper_util::client::legacy::Client<HttpsConnector<HttpConnector>, Full<Bytes>>;

#[cfg(all(
    any(feature = "async-net-rustls-tls", feature = "async-net-native-tls"),
    not(feature = "tokio-rustls-tls"),
    not(feature = "tokio-native-tls")
))]
pub type HttpClient = hyper_util::client::legacy::Client<Connector, Full<Bytes>>;

#[cfg(all(
    any(feature = "async-net-rustls-tls", feature = "async-net-native-tls"),
    not(feature = "tokio-rustls-tls"),
    not(feature = "tokio-native-tls")
))]
pub fn new_http_client() -> HttpClient {
    hyper_util::client::legacy::Client::builder(HyperExecutor).build(Connector::default())
}

#[cfg(all(
    feature = "tokio-rustls-tls",
    not(feature = "async-net-rustls-tls"),
    not(feature = "async-net-native-tls")
))]
pub fn new_http_client() -> HttpClient {
    use hyper_rustls::HttpsConnectorBuilder;

    hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(
        HttpsConnectorBuilder::new()
            .with_native_roots()
            .expect("no native root CA certificates found")
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .build(),
    )
}

#[cfg(all(
    feature = "tokio-native-tls",
    not(feature = "async-net-rustls-tls"),
    not(feature = "async-net-native-tls")
))]
pub type HttpClient =
    hyper_util::client::legacy::Client<crate::tokio_native_tls_compat::Connector, Full<Bytes>>;

#[cfg(all(
    feature = "tokio-native-tls",
    not(feature = "async-net-rustls-tls"),
    not(feature = "async-net-native-tls")
))]
pub fn new_http_client() -> HttpClient {
    use crate::tokio_native_tls_compat::Connector;

    hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(Connector::default())
}
