#[cfg(all(
    feature = "tokio-rustls-tls",
    not(feature = "async-std-rustls-tls"),
    not(feature = "async-std-native-tls")
))]
use hyper::client::HttpConnector;
#[cfg(any(
    feature = "tokio-rustls-tls",
    feature = "tokio-native-tls",
    feature = "async-std-rustls-tls",
    feature = "async-std-native-tls"
))]
use hyper::Body;
#[cfg(all(
    feature = "tokio-rustls-tls",
    not(feature = "async-std-rustls-tls"),
    not(feature = "async-std-native-tls")
))]
use hyper_rustls::HttpsConnector;

#[cfg(all(
    any(feature = "async-std-rustls-tls", feature = "async-std-native-tls"),
    not(feature = "tokio-rustls-tls"),
    not(feature = "tokio-native-tls")
))]
use crate::async_std_compat::{Connector, HyperExecutor};

#[cfg(all(
    feature = "tokio-rustls-tls",
    not(feature = "async-std-rustls-tls"),
    not(feature = "async-std-native-tls")
))]
pub type HttpClient = hyper::Client<HttpsConnector<HttpConnector>, Body>;

#[cfg(all(
    any(feature = "async-std-rustls-tls", feature = "async-std-native-tls"),
    not(feature = "tokio-rustls-tls"),
    not(feature = "tokio-native-tls")
))]
pub type HttpClient = hyper::Client<Connector, Body>;

#[cfg(all(
    any(feature = "async-std-rustls-tls", feature = "async-std-native-tls"),
    not(feature = "tokio-rustls-tls"),
    not(feature = "tokio-native-tls")
))]
pub fn new_http_client() -> HttpClient {
    hyper::Client::builder()
        .executor(HyperExecutor)
        .build(Connector)
}

#[cfg(all(
    feature = "tokio-rustls-tls",
    not(feature = "async-std-rustls-tls"),
    not(feature = "async-std-native-tls")
))]
pub fn new_http_client() -> HttpClient {
    use hyper_rustls::HttpsConnectorBuilder;

    hyper::Client::builder().build(
        HttpsConnectorBuilder::new()
            .with_native_roots()
            .https_or_http()
            .enable_http1()
            .enable_http2()
            .build(),
    )
}

#[cfg(all(
    feature = "tokio-native-tls",
    not(feature = "async-std-rustls-tls"),
    not(feature = "async-std-native-tls")
))]
pub type HttpClient = hyper::Client<crate::tokio_native_tls_compat::Connector, Body>;

#[cfg(all(
    feature = "tokio-native-tls",
    not(feature = "async-std-rustls-tls"),
    not(feature = "async-std-native-tls")
))]
pub fn new_http_client() -> HttpClient {
    use crate::tokio_native_tls_compat::Connector;

    hyper::Client::builder().build(Connector::default())
}
