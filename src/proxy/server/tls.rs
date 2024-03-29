use crate::proxy::server::proxy::Proxy;
use anyhow::{anyhow, Result};
use rcgen::{Certificate, KeyPair};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::ServerName;
use rustls::server::danger::ClientCertVerifier;
use rustls::server::WebPkiClientVerifier;
use rustls::{
    ClientConfig, DigitallySignedStruct, Error, RootCertStore, ServerConfig, SignatureScheme,
};
use rustls_pemfile::{certs, pkcs8_private_keys, rsa_private_keys};
use std::io::{BufReader, Cursor};
use std::sync::Arc;
use webpki::types::{CertificateDer, PrivateKeyDer, UnixTime};

fn get_root_cert_store() -> RootCertStore {
    let mut root_cert_store = RootCertStore::empty();
    root_cert_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    root_cert_store
}

pub(crate) fn get_self_tls_client_config() -> Result<ClientConfig> {
    let mut config = ClientConfig::builder()
        .with_root_certificates(get_root_cert_store())
        .with_no_client_auth();
    config
        .dangerous()
        .set_certificate_verifier(Arc::new(SelfSignedVerifier::new()?));
    Ok(config)
}

impl Proxy {
    pub(crate) async fn create_k8s_server_config(
        &self,
        server_name: &String,
    ) -> Result<ServerConfig> {
        let k8s_client = self.get_k8s_client(Some(server_name))?;
        let (key, cert) = k8s_client.tls_cert(server_name).await?;

        let cert = certs(&mut BufReader::new(Cursor::new(cert)))
            .filter_map(|x| x.ok())
            .collect();
        let key = rsa_private_keys(&mut BufReader::new(Cursor::new(key)))
            .next()
            .ok_or(anyhow!("Empty ingress private key"))??;

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert, key.into())?;
        Ok(config)
    }

    pub(crate) async fn create_local_server_config(
        &self,
        server_name: &String,
    ) -> Result<ServerConfig> {
        let (key, cert) = self.generate_signed_cert(server_name.as_str())?;

        let key: PrivateKeyDer = pkcs8_private_keys(&mut BufReader::new(key.as_bytes()))
            .next()
            .ok_or(anyhow!("Unable to find generated cert key"))??
            .into();

        let cert = vec![certs(&mut BufReader::new(cert.as_bytes()))
            .next()
            .ok_or(anyhow!("Unable to find generated cert pem"))??];

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert, key)?;
        Ok(config)
    }
}

pub(crate) struct CertificateData {
    pub(crate) cert: Certificate,
    pub(crate) key: KeyPair,
}

#[derive(Debug)]
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
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> std::result::Result<ServerCertVerified, Error> {
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
