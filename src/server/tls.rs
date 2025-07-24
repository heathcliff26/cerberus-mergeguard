use axum::serve::Listener;
use std::fs;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_native_tls::{
    TlsAcceptor,
    native_tls::{Identity, Protocol, TlsAcceptor as NativeTlsAcceptor},
};
use tracing::{error, warn};

type TlsStream = (tokio_native_tls::TlsStream<TcpStream>, SocketAddr);

/// Wrapper around a TcpListener that handles TLS encryption/decryption for incoming connections.
pub struct TlsListener {
    stream_rx: mpsc::Receiver<TlsStream>,
    addr: SocketAddr,
}

impl TlsListener {
    /// Read the key and cert files, bind to the given socket and handle decryption/encryption for incoming traffic.
    pub async fn bind(addr: SocketAddr, key: &str, cert: &str) -> Result<Self, TlsError> {
        let key = fs::read(key).map_err(TlsError::ReadKeyError)?;
        let cert = fs::read(cert).map_err(TlsError::ReadCertError)?;

        let id = Identity::from_pkcs8(&cert, &key).map_err(TlsError::CreateIdentityError)?;

        let tls_acceptor = NativeTlsAcceptor::builder(id)
            .min_protocol_version(Some(Protocol::Tlsv12))
            .build()
            .map_err(TlsError::CreateAcceptorError)?;

        let tls_acceptor = TlsAcceptor::from(tls_acceptor);

        let mut listener = TcpListener::bind(addr)
            .await
            .map_err(TlsError::FailedToBindListener)?;

        let addr = listener
            .local_addr()
            .map_err(TlsError::FailedToBindListener)?;

        let (stream_tx, stream_rx) = mpsc::channel(10);

        tokio::spawn(async move {
            loop {
                let tls_acceptor = tls_acceptor.clone();
                let stream_tx = stream_tx.clone();

                let (stream, addr) = Listener::accept(&mut listener).await;
                match tls_acceptor.accept(stream).await {
                    Ok(stream) => stream_tx.send((stream, addr)).await.unwrap_or_else(|e| {
                        error!("Failed to send stream to listener: {e}");
                    }),
                    Err(e) => {
                        warn!("Error during TLS handshake: {e}");
                    }
                };
            }
        });

        Ok(Self { stream_rx, addr })
    }
}

impl Listener for TlsListener {
    type Io = tokio_native_tls::TlsStream<TcpStream>;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> TlsStream {
        self.stream_rx
            .recv()
            .await
            .expect("TlsListener channel should not close before shutdown")
    }

    fn local_addr(&self) -> tokio::io::Result<Self::Addr> {
        Ok(self.addr)
    }
}

#[derive(Debug)]
pub enum TlsError {
    ReadKeyError(std::io::Error),
    ReadCertError(std::io::Error),
    CreateIdentityError(tokio_native_tls::native_tls::Error),
    CreateAcceptorError(tokio_native_tls::native_tls::Error),
    FailedToBindListener(std::io::Error),
}

impl std::fmt::Display for TlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TlsError::ReadKeyError(e) => write!(f, "Failed to read SSL key file: {e}"),
            TlsError::ReadCertError(e) => write!(f, "Failed to read SSL cert file: {e}"),
            TlsError::CreateIdentityError(e) => write!(f, "Failed to create SSL identity: {e}"),
            TlsError::CreateAcceptorError(e) => write!(f, "Failed to create SSL acceptor: {e}"),
            TlsError::FailedToBindListener(e) => write!(f, "Failed to bind listener: {e}"),
        }
    }
}

impl std::error::Error for TlsError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;

    #[test]
    fn test_tls_error_display_read_key_error() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "key file not found");
        let error = TlsError::ReadKeyError(io_error);
        let display_string = format!("{}", error);
        assert!(display_string.contains("Failed to read SSL key file"));
        assert!(display_string.contains("key file not found"));
    }

    #[test]
    fn test_tls_error_display_read_cert_error() {
        let io_error = io::Error::new(io::ErrorKind::PermissionDenied, "permission denied");
        let error = TlsError::ReadCertError(io_error);
        let display_string = format!("{}", error);
        assert!(display_string.contains("Failed to read SSL cert file"));
        assert!(display_string.contains("permission denied"));
    }

    #[test]
    fn test_tls_error_display_failed_to_bind_listener() {
        let io_error = io::Error::new(io::ErrorKind::AddrInUse, "address already in use");
        let error = TlsError::FailedToBindListener(io_error);
        let display_string = format!("{}", error);
        assert!(display_string.contains("Failed to bind listener"));
        assert!(display_string.contains("address already in use"));
    }

    #[test]
    fn test_tls_error_implements_error_trait() {
        let error = TlsError::ReadKeyError(io::Error::new(io::ErrorKind::NotFound, "test"));
        // Test that TlsError implements std::error::Error
        let _: &dyn std::error::Error = &error;
    }

    #[test]
    fn test_tls_error_debug() {
        let io_error = io::Error::new(io::ErrorKind::NotFound, "test error");
        let error = TlsError::ReadKeyError(io_error);
        let debug_string = format!("{:?}", error);
        assert!(debug_string.contains("ReadKeyError"));
    }
}
