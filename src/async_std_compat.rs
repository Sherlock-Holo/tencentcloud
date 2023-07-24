use async_std::prelude::*;
use async_std::task;

#[cfg(all(
    feature = "async-std-native-tls",
    not(feature = "async-std-rustls-tls")
))]
pub use self::native_tls_compat::{Connector, MaybeTls};
#[cfg(all(
    feature = "async-std-rustls-tls",
    not(feature = "async-std-native-tls")
))]
pub use self::rustls_compat::{Connector, MaybeTls};

#[derive(Clone, Default)]
pub struct HyperExecutor;

impl<F> hyper::rt::Executor<F> for HyperExecutor
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        task::spawn(fut);
    }
}

#[cfg(feature = "async-std-rustls-tls")]
mod rustls_compat {
    use std::future::{ready, Ready};
    use std::io;
    use std::io::ErrorKind;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use async_std::net::TcpStream;
    use async_tls::client::TlsStream;
    use async_tls::TlsConnector;
    use futures_util::future::{BoxFuture, Either};
    use futures_util::{FutureExt, TryFutureExt};
    use hyper::client::connect::{Connected, Connection};
    use hyper::service::Service;
    use hyper::Uri;
    use tokio::io::ReadBuf;
    use tokio_util::compat::{Compat, FuturesAsyncReadCompatExt};

    #[derive(Debug)]
    pub enum MaybeTls {
        Tcp(Compat<TcpStream>),
        Tls(Compat<TlsStream<TcpStream>>),
    }

    impl tokio::io::AsyncRead for MaybeTls {
        #[inline]
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let this = self.get_mut();
            match this {
                MaybeTls::Tcp(tcp) => Pin::new(tcp).poll_read(cx, buf),
                MaybeTls::Tls(tls) => Pin::new(tls).poll_read(cx, buf),
            }
        }
    }

    impl tokio::io::AsyncWrite for MaybeTls {
        #[inline]
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            let this = self.get_mut();
            match this {
                MaybeTls::Tcp(tcp) => Pin::new(tcp).poll_write(cx, buf),
                MaybeTls::Tls(tls) => Pin::new(tls).poll_write(cx, buf),
            }
        }

        #[inline]
        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            let this = self.get_mut();
            match this {
                MaybeTls::Tcp(tcp) => Pin::new(tcp).poll_flush(cx),
                MaybeTls::Tls(tls) => Pin::new(tls).poll_flush(cx),
            }
        }

        #[inline]
        fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            let this = self.get_mut();
            match this {
                MaybeTls::Tcp(tcp) => Pin::new(tcp).poll_shutdown(cx),
                MaybeTls::Tls(tls) => Pin::new(tls).poll_shutdown(cx),
            }
        }
    }

    impl Connection for MaybeTls {
        fn connected(&self) -> Connected {
            Connected::new()
        }
    }

    #[derive(Default, Clone)]
    pub struct Connector {
        tls_connector: TlsConnector,
    }

    impl Service<Uri> for Connector {
        type Response = MaybeTls;
        type Error = io::Error;
        type Future = Either<
            BoxFuture<'static, Result<Self::Response, Self::Error>>,
            Ready<Result<Self::Response, Self::Error>>,
        >;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: Uri) -> Self::Future {
            let scheme = match req.scheme_str() {
                None => {
                    return ready(Err(io::Error::new(ErrorKind::Other, "miss scheme")))
                        .right_future();
                }
                Some(scheme) => scheme,
            };
            let host = match req.host() {
                None => {
                    return ready(Err(io::Error::new(ErrorKind::Other, "miss host")))
                        .right_future();
                }
                Some(host) => host,
            };

            match scheme {
                "http" => {
                    let port = req.port_u16().unwrap_or(80);
                    let host = host.to_string();

                    async move {
                        TcpStream::connect((host.as_str(), port))
                            .map_ok(|stream| MaybeTls::Tcp(stream.compat()))
                            .await
                    }
                    .boxed()
                    .left_future()
                }

                "https" => {
                    let port = req.port_u16().unwrap_or(443);
                    let host = host.to_string();
                    let tls_connector = self.tls_connector.clone();

                    async move {
                        let tcp_stream = TcpStream::connect((host.as_str(), port)).await?;
                        let tls_stream = tls_connector.connect(host, tcp_stream).await?;

                        Ok(MaybeTls::Tls(tls_stream.compat()))
                    }
                    .boxed()
                    .left_future()
                }

                scheme => ready(Err(io::Error::new(
                    ErrorKind::Other,
                    format!("invalid scheme: {scheme}"),
                )))
                .right_future(),
            }
        }
    }
}

#[cfg(feature = "async-std-native-tls")]
mod native_tls_compat {
    use std::future::{ready, Ready};
    use std::io;
    use std::io::ErrorKind;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use async_native_tls::{TlsConnector, TlsStream};
    use async_std::net::TcpStream;
    use futures_util::future::{BoxFuture, Either};
    use futures_util::{FutureExt, TryFutureExt};
    use hyper::client::connect::{Connected, Connection};
    use hyper::service::Service;
    use hyper::Uri;
    use tokio::io::ReadBuf;
    use tokio_util::compat::{Compat, FuturesAsyncReadCompatExt};

    #[derive(Debug)]
    pub enum MaybeTls {
        Tcp(Compat<TcpStream>),
        Tls(Compat<TlsStream<TcpStream>>),
    }

    impl tokio::io::AsyncRead for MaybeTls {
        #[inline]
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &mut ReadBuf<'_>,
        ) -> Poll<io::Result<()>> {
            let this = self.get_mut();
            match this {
                MaybeTls::Tcp(tcp) => Pin::new(tcp).poll_read(cx, buf),
                MaybeTls::Tls(tls) => Pin::new(tls).poll_read(cx, buf),
            }
        }
    }

    impl tokio::io::AsyncWrite for MaybeTls {
        #[inline]
        fn poll_write(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            buf: &[u8],
        ) -> Poll<io::Result<usize>> {
            let this = self.get_mut();
            match this {
                MaybeTls::Tcp(tcp) => Pin::new(tcp).poll_write(cx, buf),
                MaybeTls::Tls(tls) => Pin::new(tls).poll_write(cx, buf),
            }
        }

        #[inline]
        fn poll_flush(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            let this = self.get_mut();
            match this {
                MaybeTls::Tcp(tcp) => Pin::new(tcp).poll_flush(cx),
                MaybeTls::Tls(tls) => Pin::new(tls).poll_flush(cx),
            }
        }

        #[inline]
        fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
            let this = self.get_mut();
            match this {
                MaybeTls::Tcp(tcp) => Pin::new(tcp).poll_shutdown(cx),
                MaybeTls::Tls(tls) => Pin::new(tls).poll_shutdown(cx),
            }
        }
    }

    impl Connection for MaybeTls {
        fn connected(&self) -> Connected {
            Connected::new()
        }
    }

    #[derive(Default, Clone)]
    pub struct Connector;

    impl Service<Uri> for Connector {
        type Response = MaybeTls;
        type Error = io::Error;
        type Future = Either<
            BoxFuture<'static, Result<Self::Response, Self::Error>>,
            Ready<Result<Self::Response, Self::Error>>,
        >;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: Uri) -> Self::Future {
            let scheme = match req.scheme_str() {
                None => {
                    return ready(Err(io::Error::new(ErrorKind::Other, "miss scheme")))
                        .right_future();
                }
                Some(scheme) => scheme,
            };
            let host = match req.host() {
                None => {
                    return ready(Err(io::Error::new(ErrorKind::Other, "miss host")))
                        .right_future();
                }
                Some(host) => host,
            };

            match scheme {
                "http" => {
                    let port = req.port_u16().unwrap_or(80);
                    let host = host.to_string();

                    async move {
                        TcpStream::connect((host.as_str(), port))
                            .map_ok(|stream| MaybeTls::Tcp(stream.compat()))
                            .await
                    }
                    .boxed()
                    .left_future()
                }

                "https" => {
                    let port = req.port_u16().unwrap_or(443);
                    let host = host.to_string();
                    let tls_connector = TlsConnector::new();

                    async move {
                        let tcp_stream = TcpStream::connect((host.as_str(), port)).await?;
                        let tls_stream = tls_connector
                            .connect(host, tcp_stream)
                            .await
                            .map_err(|err| io::Error::new(ErrorKind::Other, err))?;

                        Ok(MaybeTls::Tls(tls_stream.compat()))
                    }
                    .boxed()
                    .left_future()
                }

                scheme => ready(Err(io::Error::new(
                    ErrorKind::Other,
                    format!("invalid scheme: {scheme}"),
                )))
                .right_future(),
            }
        }
    }
}
