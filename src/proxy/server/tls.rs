use std::fs::File;
use std::{env, io};
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use rustls_pemfile::{certs, pkcs8_private_keys};
use tokio_rustls::rustls::{Certificate, PrivateKey};
use tokio_rustls::{rustls, TlsAcceptor};
use crate::util::Result;

pub fn load_certs(path: &Path) -> io::Result<Vec<Certificate>> {
    certs(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid cert"))
        .map(|mut certs| certs.drain(..).map(Certificate).collect())
}

pub fn load_keys(path: &Path) -> io::Result<Vec<PrivateKey>> {
    pkcs8_private_keys(&mut BufReader::new(File::open(path)?))
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid key"))
        .map(|mut keys| keys.drain(..).map(PrivateKey).collect())
}

pub async fn tls_acceptor(cert_path: &str, key_path: &str) -> Result<Option<TlsAcceptor>> {
    if cert_path.eq("") && key_path.eq("") {
        return Ok(None);
    }

    let parent = env::current_dir()?;


    let cert_path = parent.join(cert_path).to_str().unwrap().to_string();
    let key_path = parent.join(key_path).to_str().unwrap().to_string();

    let certs = tokio::task::spawn_blocking(move || {
        return load_certs(Path::new(&cert_path));
    }).await??;

    let mut keys = tokio::task::spawn_blocking(move || {
        let keys = load_keys(Path::new(&key_path));
        keys
    }).await??;
    let config = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, keys.remove(0))
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
    return Ok(Some(TlsAcceptor::from(Arc::new(config))));
}