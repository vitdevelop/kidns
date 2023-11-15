use crate::util::Result;
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::server::danger::ClientCertVerifier;
use rustls::server::WebPkiClientVerifier;
use rustls::{ClientConfig, DigitallySignedStruct, Error, RootCertStore, ServerName, SignatureScheme};
use rustls_pemfile::{certs, rsa_private_keys};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use std::{env, io};
use tokio_rustls::TlsAcceptor;
use webpki::types::{CertificateDer, PrivateKeyDer, UnixTime};

fn load_certs(path: &Path) -> io::Result<Vec<CertificateDer<'static>>> {
    certs(&mut BufReader::new(File::open(path)?)).collect()
}

fn load_keys(path: &Path) -> io::Result<PrivateKeyDer<'static>> {
    let file = File::open(path)?;
    rsa_private_keys(&mut BufReader::new(file))
        .next()
        .unwrap()
        .map(Into::into)
}

pub async fn tls_acceptor(cert_path: &str, key_path: &str) -> Result<Option<TlsAcceptor>> {
    if cert_path.eq("") && key_path.eq("") {
        return Ok(None);
    }

    let parent = env::current_dir()?;

    let cert_path = parent.join(cert_path).to_str().unwrap().to_string();
    let key_path = parent.join(key_path).to_str().unwrap().to_string();

    let certs = load_certs(cert_path.as_ref())?;
    let key = load_keys(key_path.as_ref())?;
    let config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
    return Ok(Some(TlsAcceptor::from(Arc::new(config))));
}

fn get_root_cert_store() -> RootCertStore {
    let mut root_cert_store = RootCertStore::empty();
    root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    root_cert_store
}

pub(crate) fn get_tls_client_config() -> Result<ClientConfig> {
    let mut config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(get_root_cert_store())
        .with_no_client_auth();
    config
        .dangerous()
        .set_certificate_verifier(Arc::new(SelfSignedVerifier::new()?));
    Ok(config)
}

pub(crate) struct SelfSignedVerifier {
    verifier: Arc<dyn ClientCertVerifier>,
}

impl SelfSignedVerifier {
    pub(crate) fn new() -> Result<SelfSignedVerifier> {
        let root_cert_store = get_root_cert_store();

        let verifier = WebPkiClientVerifier::builder(Arc::new(root_cert_store)).build()?;
        return Ok(SelfSignedVerifier { verifier });
    }
}

impl ServerCertVerifier for SelfSignedVerifier {
    fn verify_server_cert(&self, _end_entity: &CertificateDer<'_>, _intermediates: &[CertificateDer<'_>], _server_name: &ServerName, _ocsp_response: &[u8], _now: UnixTime) -> std::result::Result<ServerCertVerified, Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, Error> {
        self.verifier.verify_tls12_signature(message, cert, dss)
    }

    fn verify_tls13_signature(
        &self,
        message: &[u8],
        cert: &CertificateDer<'_>,
        dss: &DigitallySignedStruct,
    ) -> std::result::Result<HandshakeSignatureValid, Error> {
        self.verifier.verify_tls13_signature(message, cert, dss)
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.verifier.supported_verify_schemes()
    }
}
