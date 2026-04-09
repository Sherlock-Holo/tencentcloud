#[cfg(all(
    feature = "async-net-native-tls",
    not(feature = "async-net-rustls-tls")
))]
pub use self::native_tls_compat::Connector;
#[cfg(all(
    feature = "async-net-rustls-tls",
    not(feature = "async-net-native-tls")
))]
pub use self::rustls_compat::Connector;

#[derive(Clone, Default)]
pub struct HyperExecutor;

impl<F> hyper::rt::Executor<F> for HyperExecutor
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    fn execute(&self, fut: F) {
        async_global_executor::spawn(fut).detach();
    }
}

#[cfg(feature = "async-net-rustls-tls")]
mod rustls_compat {
    use std::future::ready;
    use std::io;
    use std::pin::Pin;
    use std::sync::Arc;
    use std::task::{Context, Poll, ready};

    use async_net::TcpStream;
    use futures_rustls::TlsConnector;
    use futures_rustls::client::TlsStream;
    use futures_rustls::pki_types::ServerName;
    use futures_rustls::rustls::{ClientConfig, RootCertStore};
    use futures_util::future::BoxFuture;
    use futures_util::{AsyncRead, AsyncWrite, FutureExt, TryFutureExt};
    use hyper::Uri;
    use hyper::rt::{Read, ReadBufCursor, Write};
    use hyper_util::client::legacy::connect::{Connected, Connection};
    use tower_service::Service;

    #[derive(Debug)]
    pub enum MaybeTls {
        Tcp(TcpStream),
        Tls(Box<TlsStream<TcpStream>>),
    }

    impl Read for MaybeTls {
        #[inline]
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            mut buf: ReadBufCursor<'_>,
        ) -> Poll<io::Result<()>> {
            let this = self.get_mut();
            match this {
                MaybeTls::Tcp(tcp) => {
                    let buf_slice = unsafe { buf.as_mut().assume_init_mut() };
                    let n = ready!(Pin::new(tcp).poll_read(cx, buf_slice))?;

                    unsafe {
                        buf.advance(n);
                    }

                    Poll::Ready(Ok(()))
                }

                MaybeTls::Tls(tls) => {
                    let buf_slice = unsafe { buf.as_mut().assume_init_mut() };
                    let n = ready!(Pin::new(tls).poll_read(cx, buf_slice))?;

                    unsafe {
                        buf.advance(n);
                    }

                    Poll::Ready(Ok(()))
                }
            }
        }
    }

    impl Write for MaybeTls {
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
                MaybeTls::Tcp(tcp) => Pin::new(tcp).poll_close(cx),
                MaybeTls::Tls(tls) => Pin::new(tls).poll_close(cx),
            }
        }
    }

    impl Connection for MaybeTls {
        fn connected(&self) -> Connected {
            Connected::new()
        }
    }

    #[derive(Clone)]
    pub struct Connector {
        tls_connector: TlsConnector,
    }

    impl Default for Connector {
        fn default() -> Self {
            let certs = rustls_native_certs::load_native_certs().expect("load native certs failed");
            let mut root_cert_store = RootCertStore::empty();
            for cert in certs {
                root_cert_store
                    .add(cert)
                    .unwrap_or_else(|err| panic!("add root cert failed: {err}"));
            }

            let mut client_config = ClientConfig::builder()
                .with_root_certificates(root_cert_store)
                .with_no_client_auth();

            // enable http1 and http2
            client_config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

            Self {
                tls_connector: Arc::new(client_config).into(),
            }
        }
    }

    impl Service<Uri> for Connector {
        type Response = MaybeTls;
        type Error = io::Error;
        type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: Uri) -> Self::Future {
            let scheme = match req.scheme_str() {
                None => {
                    return ready(Err(io::Error::other("miss scheme"))).boxed();
                }
                Some(scheme) => scheme,
            };
            let host = match req.host() {
                None => {
                    return ready(Err(io::Error::other("miss host"))).boxed();
                }
                Some(host) => host,
            };

            match scheme {
                "http" => {
                    let port = req.port_u16().unwrap_or(80);
                    let host = host.to_string();

                    async move {
                        TcpStream::connect((host.as_str(), port))
                            .map_ok(|stream| MaybeTls::Tcp(stream))
                            .await
                    }
                    .boxed()
                }

                "https" => {
                    let port = req.port_u16().unwrap_or(443);
                    let tls_connector = self.tls_connector.clone();
                    let server_name = match ServerName::try_from(host) {
                        Err(err) => {
                            return ready(Err(io::Error::other(err))).boxed();
                        }
                        Ok(server_name) => server_name.to_owned(),
                    };
                    let host = host.to_string();

                    async move {
                        let tcp_stream = TcpStream::connect((host, port)).await?;
                        let tls_stream = tls_connector.connect(server_name, tcp_stream).await?;

                        Ok(MaybeTls::Tls(Box::new(tls_stream)))
                    }
                    .boxed()
                }

                scheme => ready(Err(io::Error::other(format!("invalid scheme: {scheme}")))).boxed(),
            }
        }
    }
}

#[cfg(feature = "async-net-native-tls")]
mod native_tls_compat {
    use std::future::ready;
    use std::io;
    use std::pin::Pin;
    use std::task::{Context, Poll, ready};

    use async_native_tls::{TlsConnector, TlsStream};
    use async_net::TcpStream;
    use futures_util::future::BoxFuture;
    use futures_util::{AsyncRead, AsyncWrite, FutureExt, TryFutureExt};
    use hyper::Uri;
    use hyper::rt::{Read, ReadBufCursor, Write};
    use hyper_util::client::legacy::connect::{Connected, Connection};
    use tower_service::Service;

    #[derive(Debug)]
    pub enum MaybeTls {
        Tcp(TcpStream),
        Tls(TlsStream<TcpStream>),
    }

    impl Read for MaybeTls {
        #[inline]
        fn poll_read(
            self: Pin<&mut Self>,
            cx: &mut Context<'_>,
            mut buf: ReadBufCursor<'_>,
        ) -> Poll<io::Result<()>> {
            let this = self.get_mut();
            match this {
                MaybeTls::Tcp(tcp) => {
                    let buf_slice = unsafe { buf.as_mut().assume_init_mut() };
                    let n = ready!(Pin::new(tcp).poll_read(cx, buf_slice))?;

                    unsafe {
                        buf.advance(n);
                    }

                    Poll::Ready(Ok(()))
                }

                MaybeTls::Tls(tls) => {
                    let buf_slice = unsafe { buf.as_mut().assume_init_mut() };
                    let n = ready!(Pin::new(tls).poll_read(cx, buf_slice))?;

                    unsafe {
                        buf.advance(n);
                    }

                    Poll::Ready(Ok(()))
                }
            }
        }
    }

    impl Write for MaybeTls {
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
                MaybeTls::Tcp(tcp) => Pin::new(tcp).poll_close(cx),
                MaybeTls::Tls(tls) => Pin::new(tls).poll_close(cx),
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
        _priv: (),
    }

    impl Service<Uri> for Connector {
        type Response = MaybeTls;
        type Error = io::Error;
        type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }

        fn call(&mut self, req: Uri) -> Self::Future {
            let scheme = match req.scheme_str() {
                None => {
                    return ready(Err(io::Error::other("miss scheme"))).boxed();
                }
                Some(scheme) => scheme,
            };
            let host = match req.host() {
                None => {
                    return ready(Err(io::Error::other("miss host"))).boxed();
                }
                Some(host) => host,
            };

            match scheme {
                "http" => {
                    let port = req.port_u16().unwrap_or(80);
                    let host = host.to_string();

                    async move {
                        TcpStream::connect((host.as_str(), port))
                            .map_ok(MaybeTls::Tcp)
                            .await
                    }
                    .boxed()
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
                            .map_err(io::Error::other)?;

                        Ok(MaybeTls::Tls(tls_stream))
                    }
                    .boxed()
                }

                scheme => ready(Err(io::Error::other(format!("invalid scheme: {scheme}")))).boxed(),
            }
        }
    }
}
