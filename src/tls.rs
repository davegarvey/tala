//! TLS for the network listener: a TLS 1.3 server configuration from the host
//! identity, and a client configuration that pins the host certificate.
//!
//! Clients pin the certificate's SHA-256 fingerprint instead of trusting
//! certificate authorities. Pinning alone is not enough: the handshake
//! signature must still be verified with the pinned certificate's key, or a
//! peer could replay the public certificate without holding the key.

use crate::{models::*, network};
use rustls::{
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    crypto::CryptoProvider,
    pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer, ServerName, UnixTime},
    DigitallySignedStruct, SignatureScheme,
};
use std::sync::Arc;

fn provider() -> Arc<CryptoProvider> {
    Arc::new(rustls::crypto::ring::default_provider())
}

fn tls_error(e: rustls::Error) -> Failure {
    Failure::new(
        "NETWORK_CONFIG",
        format!("TLS configuration failed: {e}"),
        "Run tala network rotate-token to create new credentials.",
    )
}

/// TLS 1.3 server configuration presenting the host certificate.
pub fn server_config(identity: &network::HostIdentity) -> Result<Arc<rustls::ServerConfig>> {
    let cert = CertificateDer::from(identity.cert_der()?);
    let key = PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(identity.key_der()?));
    let config = rustls::ServerConfig::builder_with_provider(provider())
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(tls_error)?
        .with_no_client_auth()
        .with_single_cert(vec![cert], key)
        .map_err(tls_error)?;
    Ok(Arc::new(config))
}

/// Client configuration that accepts only the pinned certificate.
pub fn pinned_client_config(fingerprint: &str) -> Result<rustls::ClientConfig> {
    let provider = provider();
    let verifier = PinnedVerifier {
        fingerprint: fingerprint.to_ascii_lowercase(),
        provider: provider.clone(),
    };
    Ok(rustls::ClientConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(tls_error)?
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(verifier))
        .with_no_client_auth())
}

#[derive(Debug)]
struct PinnedVerifier {
    fingerprint: String,
    provider: Arc<CryptoProvider>,
}

impl ServerCertVerifier for PinnedVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, rustls::Error> {
        // Name and validity dates are irrelevant: the pin is the identity.
        if network::fingerprint(end_entity.as_ref()) == self.fingerprint {
            Ok(ServerCertVerified::assertion())
        } else {
            Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::ApplicationVerificationFailure,
            ))
        }
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            cert,
            dss,
            &self.provider.signature_verification_algorithms,
        )
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.provider
            .signature_verification_algorithms
            .supported_schemes()
    }
}

/// Whether a request error was caused by a rejected server certificate.
/// The rustls error arrives wrapped in nested `io::Error`s, whose `source()`
/// skips the wrapped error, so each wrapper is unwrapped explicitly.
pub fn is_certificate_rejection(error: &(dyn std::error::Error + 'static)) -> bool {
    if let Some(rustls::Error::InvalidCertificate(_)) = error.downcast_ref::<rustls::Error>() {
        return true;
    }
    if let Some(inner) = error
        .downcast_ref::<std::io::Error>()
        .and_then(|io| io.get_ref())
    {
        if is_certificate_rejection(inner) {
            return true;
        }
    }
    error.source().is_some_and(is_certificate_rejection)
}

#[cfg(test)]
mod tests {
    use super::is_certificate_rejection;

    #[test]
    fn certificate_rejection_is_found_inside_nested_io_errors() {
        let rejected = rustls::Error::InvalidCertificate(rustls::CertificateError::BadSignature);
        let nested = std::io::Error::other(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            rejected,
        ));
        assert!(is_certificate_rejection(&nested));
        let refused = std::io::Error::from(std::io::ErrorKind::ConnectionRefused);
        assert!(!is_certificate_rejection(&refused));
    }
}
