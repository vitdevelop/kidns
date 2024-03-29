use crate::k8s::client::K8sClient;
use crate::proxy::http::get_host;
use crate::proxy::server::proxy::Proxy;
use crate::proxy::server::tls::get_self_tls_client_config;
use crate::util::{is_tls, log_error_result};
use anyhow::{anyhow, format_err, Error, Result};
use rustls::ServerConfig;
use std::io;
use std::io::ErrorKind;
use std::io::ErrorKind::UnexpectedEof;
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::try_join;
use tokio_rustls::server::TlsStream;
use tokio_rustls::{rustls, LazyConfigAcceptor, TlsConnector};

impl Proxy {
    pub async fn serve(self) -> Result<()> {
        let proxy = Arc::new(self);

        let http_proxy = proxy.clone();
        let http = tokio::spawn(async {
            let http_port = http_proxy.http_port;
            log_error_result(http_proxy.serve_port(http_port).await.map_err(|e| {
                anyhow!(
                    "Unable to run proxy on http port {}, with error {:?}",
                    http_port,
                    e
                )
            }))
        });

        let https_proxy = proxy.clone();
        let https = tokio::spawn(async {
            let https_port = https_proxy.https_port;
            log_error_result(https_proxy.serve_port(https_port).await.map_err(|e| {
                anyhow!(
                    "Unable to run proxy on https port {}, with error {:?}",
                    https_port,
                    e
                )
            }))
        });

        try_join!(http, https)?;

        Ok(())
    }

    async fn serve_port(self: Arc<Proxy>, port: u16) -> Result<()> {
        let addr = SocketAddr::from((Ipv4Addr::from_str(&self.host)?, port));

        let listener = TcpListener::bind(addr).await?;

        loop {
            let (client_conn, _) = listener.accept().await?;

            let proxy = self.clone();
            tokio::spawn(async move {
                log_error_result(proxy.forward_connection(client_conn).await);
            });
        }
    }
    pub(crate) fn get_k8s_client(&self, url: Option<&String>) -> Result<Arc<K8sClient>> {
        Ok(match url {
            None => self
                .k8s_clients
                .first()
                .ok_or(anyhow!("Unable to found any k8s client"))?
                .clone(),
            Some(url) => self
                .ingress_clients
                .get(url)
                .ok_or(anyhow!("Unable to found any k8s client for url {}", url))?
                .clone(),
        })
    }

    async fn forward_connection(self: Arc<Self>, client_conn: TcpStream) -> Result<()> {
        let is_tls = is_tls(&client_conn).await?;

        if is_tls {
            let acceptor =
                LazyConfigAcceptor::new(rustls::server::Acceptor::default(), client_conn);
            let start = acceptor.await.unwrap();
            let ch = start.client_hello();

            let server_name = ch
                .server_name()
                .ok_or(anyhow!("TLS connection didn't provide server name"))?
                .to_owned();

            if self.root_cert.is_none() {
                let server_config = self.get_k8s_server_config(&server_name).await?;

                let mut client_stream = start.into_stream(server_config).await?;

                self.proxy_tls_connection(&mut client_stream, &server_name)
                    .await?;
            } else {
                let user_defined_config = self.get_local_server_config(&server_name).await?;

                let client_stream = &mut start.into_stream(user_defined_config).await?;
                self.proxy_tls_connection(client_stream, &server_name)
                    .await?;
            }
            Ok(())
        } else {
            self.proxy_connection(client_conn).await
        }
    }

    async fn proxy_tls_connection(
        &self,
        client_stream: &mut TlsStream<TcpStream>,
        host: &String,
    ) -> Result<()> {
        let tunnel: Result<(u64, u64), io::Error> = if self.ingress_clients.contains_key(host) {
            let domain = rustls::pki_types::ServerName::try_from(host.as_str())
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dnsname"))?
                .to_owned();
            let connector = TlsConnector::from(Arc::new(get_self_tls_client_config()?));

            let k8s_forwarder = self.get_k8s_port_forwarder(Some(host), true).await?;
            let mut k8s_socket = connector.connect(domain, k8s_forwarder).await?;

            tokio::io::copy_bidirectional(client_stream, &mut k8s_socket).await
        } else {
            match self.local_clients.get(host) {
                Some(addr) => {
                    let mut local_socket = self.get_local_port_forwarder(addr).await?;
                    tokio::io::copy_bidirectional(client_stream, &mut local_socket).await
                }
                None => Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format_err!("Unable to proxy connection to {}", host),
                )),
            }
        };

        if let Err(e) = tunnel {
            if e.kind() != UnexpectedEof {
                return Err(Error::from(e));
            }
        }

        Ok(())
    }

    async fn proxy_connection(&self, mut client_conn: TcpStream) -> Result<()> {
        let url = get_host(&mut client_conn).await?;

        // remap
        let host = match url.as_str() {
            "" => None,
            _ => Some(&url),
        };

        if self.ingress_clients.contains_key(&url) {
            let mut upstream_conn = self.get_k8s_port_forwarder(host, false).await?;
            tokio::io::copy_bidirectional(&mut client_conn, &mut upstream_conn).await?;
        }

        match self.local_clients.get(&url) {
            Some(addr) => {
                let mut upstream_conn = self.get_local_port_forwarder(addr).await?;
                tokio::io::copy_bidirectional(&mut client_conn, &mut upstream_conn).await?;
            }
            None => {
                // close connection
                client_conn.shutdown().await?;
            }
        };

        Ok(())
    }
    async fn get_k8s_port_forwarder(
        &self,
        url: Option<&String>,
        secure: bool,
    ) -> Result<impl AsyncRead + AsyncWrite + Unpin> {
        let k8s_client = self.get_k8s_client(url)?;

        k8s_client.get_port_forwarder(secure).await
    }

    async fn get_local_port_forwarder(
        &self,
        addr: &SocketAddr,
    ) -> Result<impl AsyncRead + AsyncWrite + Unpin> {
        return Ok(TcpStream::connect(addr).await?);
    }

    async fn get_k8s_server_config(&self, host: &String) -> Result<Arc<ServerConfig>> {
        let certs = self.destinations_certs.read().await;
        match certs.get(host) {
            None => {
                drop(certs);
                let server_config = Arc::new(self.create_k8s_server_config(host).await?);

                self.destinations_certs
                    .write()
                    .await
                    .insert(host.to_string(), server_config.clone());
                Ok(server_config)
            }
            Some(cert_config) => Ok(cert_config.clone()),
        }
    }

    async fn get_local_server_config(&self, host: &String) -> Result<Arc<ServerConfig>> {
        let certs = self.destinations_certs.read().await;
        match certs.get(host) {
            None => {
                drop(certs);
                let user_defined_config = Arc::new(self.create_local_server_config(host).await?);

                self.destinations_certs
                    .write()
                    .await
                    .insert(host.to_string(), user_defined_config.clone());
                Ok(user_defined_config)
            }
            Some(cert_config) => Ok(cert_config.clone()),
        }
    }
}
