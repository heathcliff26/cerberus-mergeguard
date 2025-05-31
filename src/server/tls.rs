use axum::{Router, extract::Request};
use hyper::body::Incoming;
use hyper_util::rt::{TokioExecutor, TokioIo};
use std::fs;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::{pin, select};
use tokio_native_tls::{
    TlsAcceptor,
    native_tls::{Identity, Protocol, TlsAcceptor as NativeTlsAcceptor},
};
use tower_service::Service;
use tracing::{error, info, warn};

/// Create a tls acceptor from the provided key and cert files.
/// Uses native-tls for tls implementation.
pub fn native_tls_acceptor(key: &str, cert: &str) -> Result<NativeTlsAcceptor, String> {
    let key = fs::read(key).map_err(|e| format!("Failed to read SSL key file: {e}"))?;
    let cert = fs::read(cert).map_err(|e| format!("Failed to read SSL cert file: {e}"))?;

    let id = Identity::from_pkcs8(&cert, &key)
        .map_err(|e| format!("Failed to create SSL identity: {e}"))?;

    NativeTlsAcceptor::builder(id)
        .min_protocol_version(Some(Protocol::Tlsv12))
        .build()
        .map_err(|e| format!("Failed to create SSL acceptor: {e}"))
}

pub async fn serve_tls(
    listener: TcpListener,
    router: Router,
    cert: &str,
    key: &str,
    shutdown_signal: impl Future<Output = ()> + Send + 'static,
) -> Result<(), String> {
    let tls_acceptor = native_tls_acceptor(cert, key)?;
    pin!(shutdown_signal);

    loop {
        let tower_service = router.clone();
        let tls_acceptor = tls_acceptor.clone();

        let connection = listener.accept();

        select! {
            _ = &mut shutdown_signal => {
                info!("Shutting down server");
                break;
            },
            connection = connection => {
                match connection {
                    Ok((stream, addr)) => {
                        handle_connection(stream,addr, tls_acceptor.into(), tower_service);
                    }
                    Err(e) => {
                        error!("Failed to accept connection: {e}");
                    }
                }
            }
        };
    }

    Ok(())
}

fn handle_connection(stream: TcpStream, addr: SocketAddr, acceptor: TlsAcceptor, service: Router) {
    tokio::spawn(async move {
        // Wait for tls handshake to happen
        let Ok(stream) = acceptor.accept(stream).await else {
            warn!("error during tls handshake connection from {}", addr);
            return;
        };

        // Hyper has its own `AsyncRead` and `AsyncWrite` traits and doesn't use tokio.
        // `TokioIo` converts between them.
        let stream = TokioIo::new(stream);

        // Hyper also has its own `Service` trait and doesn't use tower. We can use
        // `hyper::service::service_fn` to create a hyper `Service` that calls our app through
        // `tower::Service::call`.
        let hyper_service = hyper::service::service_fn(move |request: Request<Incoming>| {
            // We have to clone `tower_service` because hyper's `Service` uses `&self` whereas
            // tower's `Service` requires `&mut self`.
            //
            // We don't need to call `poll_ready` since `Router` is always ready.
            service.clone().call(request)
        });

        let ret = hyper_util::server::conn::auto::Builder::new(TokioExecutor::new())
            .serve_connection_with_upgrades(stream, hyper_service)
            .await;

        if let Err(err) = ret {
            warn!("error serving connection from {addr}: {err}");
        }
    });
}
