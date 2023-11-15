use crate::proxy::http::get_host;
use crate::proxy::server::proxy::Proxy;
use crate::proxy::server::tls::{tls_acceptor, get_tls_client_config};
use crate::util::Result;
use k8s_openapi::api::core::v1::Pod;
use kube::{Api, ResourceExt};
use log::{debug, error, info};
use std::io;
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use std::sync::Arc;
use tokio::io::AsyncWriteExt;
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::server::TlsStream;
use tokio_rustls::{rustls, TlsConnector};

enum ServerStream {
    TcpStream(TcpStream),
    TlsStream(TlsStream<TcpStream>),
}

impl Proxy {
    pub async fn serve(self) -> Result<()> {
        let k8s_client = &self.k8s_client;

        let mut pod_list = k8s_client.pod_list().await?;
        let pod_api = k8s_client.pod_api()?;

        let acceptor = tls_acceptor(self.cert_path.as_str(), self.key_path.as_str()).await?;

        return if let Some(pod) = pod_list.items.pop() {
            let addr = SocketAddr::from((Ipv4Addr::from_str(&self.host)?, self.port));
            let pod_name = pod.name_any();
            debug!("Connect to {}", pod_name);

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

                let pod_api = pod_api.to_owned();
                let pod_name = pod_name.to_string();
                let pod_port = k8s_client.pod_port;

                tokio::spawn(async move {
                    match forward_connection(&pod_api, pod_name.as_str(), pod_port, client_conn)
                        .await
                    {
                        Ok(_) => {}
                        Err(e) => error!("Error on handle client, err: {:?}", e),
                    };
                });
            }
        } else {
            Err(format!(
                "Pods in namespace {} with label {} not found",
                k8s_client.pod_namespace, k8s_client.pod_label
            )
            .into())
        };
    }
}

async fn forward_connection(
    pods: &Api<Pod>,
    pod_name: &str,
    port: u16,
    mut client_conn: ServerStream,
) -> Result<()> {
    let mut forwarder = pods.portforward(pod_name, &[port]).await?;
    let mut upstream_conn = forwarder
        .take_stream(port)
        .ok_or("Cannot get stream from port forward")?;

    match &mut client_conn {
        ServerStream::TcpStream(stream) => {
            tokio::io::copy_bidirectional(stream, &mut upstream_conn).await?;
        }
        ServerStream::TlsStream(stream) => {
            let connector = TlsConnector::from(Arc::new(get_tls_client_config()?));
            //
            let (host, data) = get_host(stream).await?;

            debug!("Route request to {}", host);

            let domain = rustls::ServerName::try_from(host.as_str())
                .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "invalid dnsname"))?;
            let mut socket = connector.connect(domain, upstream_conn).await?;

            socket.write_all(data.as_slice()).await?;

            tokio::io::copy_bidirectional(stream, &mut socket).await?;
        }
    }

    Ok(())
}
