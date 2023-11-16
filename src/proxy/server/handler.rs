use crate::proxy::http::get_host;
use crate::proxy::server::proxy::Proxy;
use crate::proxy::server::tls::{get_tls_client_config, tls_acceptor};
use crate::util::Result;
use kube::ResourceExt;
use log::{debug, error, info};
use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::server::TlsStream;
use tokio_rustls::{rustls, TlsConnector};

enum ServerStream {
    TcpStream(TcpStream),
    TlsStream(TlsStream<TcpStream>),
}

impl Proxy {
    pub async fn serve(self) -> Result<()> {
        let proxy = Arc::new(self);

        let acceptor = tls_acceptor(proxy.cert_path.as_str(), proxy.key_path.as_str()).await?;

        let addr = SocketAddr::from((Ipv4Addr::from_str(&proxy.host)?, proxy.port));

        let listener = TcpListener::bind(addr).await?;

        info!("Proxy Server Initialized");
        // keep the server running
        loop {
            let (client_conn, _) = listener.accept().await?;

            let client_conn = match acceptor.to_owned() {
                None => ServerStream::TcpStream(client_conn),
                Some(acc) => {
                    let stream = match acc.accept(client_conn).await {
                        Ok(stream) => stream,
                        Err(err) => {
                            error!("{}", err);
                            continue;
                        }
                    };
                    ServerStream::TlsStream(stream)
                }
            };

            let proxy = proxy.clone();
            tokio::spawn(async move {
                match proxy.forward_connection(client_conn).await {
                    Ok(_) => {}
                    Err(e) => error!("Error on handle client, err: {:?}", e),
                };
            });
        }
    }

    async fn get_k8s_port_forwarder(
        &self,
        url: Option<&String>,
    ) -> Result<impl AsyncRead + AsyncWrite + Unpin> {
        let k8s_client = match url {
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
        };

        let mut pod_list = k8s_client.pod_list().await?;
        let pod_api = k8s_client.pod_api()?;

        let pod = pod_list.items.pop().ok_or("Unable to find free pod port")?;
        let pod_name = pod.name_any();
        debug!("Connect to {}", pod_name);

        let pod_port = k8s_client.pod_port;

        let mut forwarder = pod_api.portforward(pod_name.as_str(), &[pod_port]).await?;
        let upstream_conn = forwarder
            .take_stream(pod_port)
            .ok_or("Cannot get stream from port forward")?;

        Ok(upstream_conn)
    }

    async fn forward_connection(self: Arc<Self>, mut client_conn: ServerStream) -> Result<()> {
        match &mut client_conn {
            ServerStream::TcpStream(stream) => {
                let mut upstream_conn = self.get_k8s_port_forwarder(None).await?;
                tokio::io::copy_bidirectional(stream, &mut upstream_conn).await?;
            }
            ServerStream::TlsStream(stream) => {
                self.proxy_tls_connection(stream).await?;
            }
        }

        Ok(())
    }

    async fn proxy_tls_connection(
        &self,
        downstream: &mut TlsStream<TcpStream>,
    ) -> Result<()> {
        let connector = TlsConnector::from(Arc::new(get_tls_client_config()?));
        //
        let (host, data) = get_host(downstream).await?;

        debug!("Route request to {}", host);

        let upstream = self.get_k8s_port_forwarder(Some(&host)).await?;

        let domain = rustls::ServerName::try_from(host.as_str())
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dnsname"))?;
        let mut socket = connector.connect(domain, upstream).await?;

        socket.write_all(data.as_slice()).await?;

        tokio::io::copy_bidirectional(downstream, &mut socket).await?;

        Ok(())
    }
}
