use crate::k8s::client::K8sClient;
use crate::proxy::http::get_host;
use crate::proxy::server::proxy::{DestinationConfig, Proxy};
use crate::proxy::server::tls::get_self_tls_client_config;
use crate::util::{is_tls, log_error_result};
use anyhow::{anyhow, Error, Result};
use std::io;
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
            log_error_result(http_proxy.serve_port(http_port).await)
        });

        let https_proxy = proxy.clone();
        let https = tokio::spawn(async {
            let https_port = https_proxy.https_port;
            log_error_result(https_proxy.serve_port(https_port).await)
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
                log_error_result(proxy.forward_k8s_connection(client_conn).await);
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

    async fn forward_k8s_connection(self: Arc<Self>, client_conn: TcpStream) -> Result<()> {
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

                let mut client_stream = start.into_stream(server_config.server_config).await?;

                self.proxy_tls_connection(
                    &mut client_stream,
                    &server_name,
                    server_config.k8s_client.is_some(),
                )
                .await?;
            } else {
                let user_defined_config = self.get_local_server_config(&server_name).await?;

                let client_stream =
                    &mut start.into_stream(user_defined_config.server_config).await?;
                self.proxy_tls_connection(
                    client_stream,
                    &server_name,
                    user_defined_config.k8s_client.is_some(),
                )
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
        is_k8s: bool,
    ) -> Result<()> {
        let tunnel = match is_k8s {
            true => {
                let domain = rustls::pki_types::ServerName::try_from(host.as_str())
                    .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dnsname"))?
                    .to_owned();
                let connector = TlsConnector::from(Arc::new(get_self_tls_client_config()?));

                let k8s_forwarder = self.get_k8s_port_forwarder(Some(host), true).await?;
                let mut k8s_socket = connector.connect(domain, k8s_forwarder).await?;

                tokio::io::copy_bidirectional(client_stream, &mut k8s_socket).await
            }
            false => {
                let mut local_socket = self.get_local_port_forwarder(host).await?;
                tokio::io::copy_bidirectional(client_stream, &mut local_socket).await
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

        let is_k8s = self.ingress_clients.contains_key(&url);

        if is_k8s {
            let mut upstream_conn = self.get_k8s_port_forwarder(host, false).await?;
            tokio::io::copy_bidirectional(&mut client_conn, &mut upstream_conn).await?;
        } else {
            if let Some(url) = host {
                let mut upstream_conn = self.get_local_port_forwarder(url).await?;
                tokio::io::copy_bidirectional(&mut client_conn, &mut upstream_conn).await?;
            } else {
                // close connection
                client_conn.shutdown().await?;
            }
        }

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
        url: &String,
    ) -> Result<impl AsyncRead + AsyncWrite + Unpin> {
        return Ok(TcpStream::connect(url).await?);
    }

    async fn get_k8s_server_config(&self, host: &String) -> Result<DestinationConfig> {
        let certs = self.destinations_certs.read().await;
        match certs.get(host) {
            None => {
                drop(certs);
                let server_config = Arc::new(self.create_k8s_server_config(host).await?);
                let k8s_client = self.get_k8s_client(Some(host))?;

                self.destinations_certs.write().await.insert(
                    host.to_string(),
                    DestinationConfig::new(server_config.clone(), Some(k8s_client.clone())),
                );
                Ok(DestinationConfig::new(server_config, Some(k8s_client)))
            }
            Some(cert_config) => Ok(cert_config.clone()),
        }
    }

    async fn get_local_server_config(&self, host: &String) -> Result<DestinationConfig> {
        let certs = self.destinations_certs.read().await;
        match certs.get(host) {
            None => {
                drop(certs);
                let user_defined_config = Arc::new(self.create_local_server_config(host).await?);
                let k8s_client = self.get_k8s_client(Some(host))?;

                self.destinations_certs.write().await.insert(
                    host.to_string(),
                    DestinationConfig::new(user_defined_config.clone(), Some(k8s_client.clone())),
                );
                Ok(DestinationConfig::new(
                    user_defined_config,
                    Some(k8s_client),
                ))
            }
            Some(cert_config) => Ok(cert_config.clone()),
        }
    }
}
