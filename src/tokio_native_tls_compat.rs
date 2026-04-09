use std::future::{Ready, ready};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use futures_util::future::{BoxFuture, Either};
use futures_util::{FutureExt, TryFutureExt};
use hyper::Uri;
use hyper::rt::{Read, ReadBufCursor, Write};
use hyper_util::client::legacy::connect::{Connected, Connection};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tokio_native_tls::{TlsConnector, TlsStream};
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
                let mut tokio_buf = unsafe { ReadBuf::uninit(buf.as_mut()) };
                ready!(AsyncRead::poll_read(Pin::new(tcp), cx, &mut tokio_buf))?;
                let n = tokio_buf.filled().len();

                unsafe {
                    buf.advance(n);
                }

                Poll::Ready(Ok(()))
            }

            MaybeTls::Tls(tls) => {
                let mut tokio_buf = unsafe { ReadBuf::uninit(buf.as_mut()) };
                ready!(AsyncRead::poll_read(Pin::new(tls), cx, &mut tokio_buf))?;
                let n = tokio_buf.filled().len();

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
            MaybeTls::Tcp(tcp) => Pin::new(tcp).poll_shutdown(cx),
            MaybeTls::Tls(tls) => Pin::new(tls).poll_shutdown(cx),
        }
    }
}

impl Connection for MaybeTls {
    fn connected(&self) -> Connected {
        match self {
            MaybeTls::Tcp(s) => s.connected(),
            MaybeTls::Tls(s) => {
                let tls = s.get_ref();
                let connected = tls.get_ref().get_ref().connected();
                match tls.negotiated_alpn() {
                    Ok(Some(alpn)) if alpn == b"h2" => connected.negotiated_h2(),
                    _ => connected,
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Connector {
    tls_connector: TlsConnector,
}

impl Default for Connector {
    fn default() -> Self {
        let mut builder = native_tls::TlsConnector::builder();
        let tls_connector = builder
            .request_alpns(&["h2", "http/1.1"])
            .build()
            .unwrap_or_else(|err| panic!("build tls connector failed: {err}"));

        Self {
            tls_connector: tls_connector.into(),
        }
    }
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
                return ready(Err(io::Error::other("miss scheme"))).right_future();
            }
            Some(scheme) => scheme,
        };
        let host = match req.host() {
            None => {
                return ready(Err(io::Error::other("miss host"))).right_future();
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
                .left_future()
            }

            "https" => {
                let port = req.port_u16().unwrap_or(443);
                let host = host.to_string();
                let tls_connector = self.tls_connector.clone();

                async move {
                    let tcp_stream = TcpStream::connect((host.as_str(), port)).await?;
                    let tls_stream = tls_connector
                        .connect(&host, tcp_stream)
                        .await
                        .map_err(io::Error::other)?;

                    Ok(MaybeTls::Tls(tls_stream))
                }
                .boxed()
                .left_future()
            }

            scheme => {
                ready(Err(io::Error::other(format!("invalid scheme: {scheme}")))).right_future()
            }
        }
    }
}
