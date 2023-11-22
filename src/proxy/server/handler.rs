use crate::k8s::client::K8sClient;
use crate::proxy::http::get_host;
use crate::proxy::server::proxy::Proxy;
use crate::proxy::server::tls::{get_self_tls_client_config, get_self_tls_server_config};
use crate::util::{is_tls, log_error_result, Result};
use std::io;
use std::io::ErrorKind::UnexpectedEof;
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio::try_join;
use tokio_rustls::server::TlsStream;
use tokio_rustls::{rustls, LazyConfigAcceptor, StartHandshake, TlsConnector};

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
                if proxy.k8s_clients.len() > 1 {
                    log_error_result(proxy.forward_multi_cluster_connection(client_conn).await);
                } else {
                    log_error_result(proxy.forward_single_cluster_connection(client_conn).await);
                }
            });
        }
    }
    pub(crate) fn get_k8s_client(&self, url: Option<&String>) -> Result<Arc<K8sClient>> {
        Ok(match url {
            None => self
                .k8s_clients
                .first()
                .ok_or("Unable to found any k8s client")?
                .clone(),
            Some(url) => self
                .ingress_clients
                .get(url)
                .ok_or(format!("Unable to found any k8s client for url {}", url))?
                .clone(),
        })
    }

    async fn get_k8s_port_forwarder(
        &self,
        url: Option<&String>,
        secure: bool,
    ) -> Result<impl AsyncRead + AsyncWrite + Unpin> {
        let k8s_client = self.get_k8s_client(url)?;

        k8s_client.get_port_forwarder(secure).await
    }

    async fn forward_multi_cluster_connection(
        self: Arc<Self>,
        mut client_conn: TcpStream,
    ) -> Result<()> {
        let is_tls = is_tls(&client_conn).await?;

        if is_tls {
            let acceptor =
                LazyConfigAcceptor::new(rustls::server::Acceptor::default(), client_conn);
            let start = acceptor.await.unwrap();
            let ch = start.client_hello();

            let server_name = ch
                .server_name()
                .ok_or("TLS connection didn't provide server name")?
                .to_owned();

            match get_self_tls_server_config(self.cert_path.as_str(), self.key_path.as_str())
                .await?
            {
                None => self.forward_tls_connection(start, &server_name).await?,
                Some(user_defined_config) => {
                    let client_stream =
                        &mut start.into_stream(Arc::new(user_defined_config)).await?;
                    self.proxy_tls_connection(client_stream, &server_name)
                        .await?
                }
            }
            Ok(())
        } else {
            let host = get_host(&mut client_conn).await?;

            let server_stream = &mut self.get_k8s_port_forwarder(Some(&host), false).await?;

            tokio::io::copy_bidirectional(&mut client_conn, server_stream).await?;

            Ok(())
        }
    }

    async fn forward_single_cluster_connection(
        self: Arc<Self>,
        mut client_conn: TcpStream,
    ) -> Result<()> {
        let is_tls = is_tls(&mut client_conn).await?;
        let mut upstream_conn = self.get_k8s_port_forwarder(None, is_tls).await?;
        tokio::io::copy_bidirectional(&mut client_conn, &mut upstream_conn).await?;

        Ok(())
    }

    async fn forward_tls_connection(
        &self,
        client_handshake: StartHandshake<TcpStream>,
        host: &String,
    ) -> Result<()> {
        let server_config = {
            let certs = self.ingress_certs.read().await;
            match certs.get(host) {
                None => {
                    drop(certs);
                    let server_config = Arc::new(self.get_tls_server_config(host).await?);

                    self.ingress_certs
                        .write()
                        .await
                        .insert(host.to_string(), server_config.clone());
                    server_config
                }
                Some(cert_config) => cert_config.clone(),
            }
        };

        let mut client_stream = client_handshake.into_stream(server_config).await?;

        self.proxy_tls_connection(&mut client_stream, host).await?;

        Ok(())
    }

    async fn proxy_tls_connection(
        &self,
        client_stream: &mut TlsStream<TcpStream>,
        host: &String,
    ) -> Result<()> {
        let connector = TlsConnector::from(Arc::new(get_self_tls_client_config()?));

        let upstream = self.get_k8s_port_forwarder(Some(host), true).await?;

        let domain = rustls::ServerName::try_from(host.as_str())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dnsname"))?;
        let mut socket = connector.connect(domain, upstream).await?;

        if let Err(e) = tokio::io::copy_bidirectional(client_stream, &mut socket).await {
            if e.kind() != UnexpectedEof {
                return Err(Box::new(e));
            }
        }

        Ok(())
    }
}
