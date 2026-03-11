use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::CertificateDer;
use rustls::{DigitallySignedStruct, SignatureScheme};

/// A struct representing an entity that accepts all SSL/TLS certificates.
///
/// # Description
/// `AcceptAllCertificates` is typically used in scenarios where SSL/TLS certificate validation
/// needs to be bypassed (e.g., for debugging or testing purposes).
/// Using this in production environments is **strongly discouraged** as it poses a security risk
/// by allowing connections to potentially untrusted or malicious servers.
///
/// # Derive Attributes
/// - `Debug`: Enables formatting the struct for debugging purposes.
/// - `Clone`: Allows creating a duplicate of an existing instance of this struct.
///
/// # Examples
/// ```
/// let cert_validator = AcceptAllCertificates;
/// println!("{:?}", cert_validator); // Debug output of the struct
/// let cloned_validator = cert_validator.clone(); // Create a clone
/// ```
///
/// # Warning
/// Ensure careful use of this struct. Inappropriate usage could lead to
/// significant security vulnerabilities.
#[derive(Debug, Clone)]
pub struct AcceptAllCertificates;

impl ServerCertVerifier for AcceptAllCertificates {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        // Return all schemes you want to support
        //  RSA_PKCS1_SHA1 => 0x0201,
        //         ECDSA_SHA1_Legacy => 0x0203,
        //         RSA_PKCS1_SHA256 => 0x0401,
        //         ECDSA_NISTP256_SHA256 => 0x0403,
        //         RSA_PKCS1_SHA384 => 0x0501,
        //         ECDSA_NISTP384_SHA384 => 0x0503,
        //         RSA_PKCS1_SHA512 => 0x0601,
        //         ECDSA_NISTP521_SHA512 => 0x0603,
        //         RSA_PSS_SHA256 => 0x0804,
        //         RSA_PSS_SHA384 => 0x0805,
        //         RSA_PSS_SHA512 => 0x0806,
        //         ED25519 => 0x0807,
        //         ED448 => 0x0808,
        //         // https://datatracker.ietf.org/doc/html/draft-ietf-tls-mldsa-00#name-iana-considerations
        //         ML_DSA_44 => 0x0904,
        //         ML_DSA_65 => 0x0905,
        //         ML_DSA_87 => 0x0906,
        vec![
            SignatureScheme::ECDSA_SHA1_Legacy,
            SignatureScheme::RSA_PKCS1_SHA256,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::RSA_PKCS1_SHA384,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::RSA_PKCS1_SHA512,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ED25519,
            SignatureScheme::ED448,
            SignatureScheme::ML_DSA_44,
            SignatureScheme::ML_DSA_65,
            SignatureScheme::ML_DSA_87,
        ]
    }
}
