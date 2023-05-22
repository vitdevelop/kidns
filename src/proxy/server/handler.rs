use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use futures::TryStreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::{Api, ResourceExt};
use log::{debug, error, info};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::TlsAcceptor;
use tokio_stream::wrappers::TcpListenerStream;
use crate::proxy::server::proxy::Proxy;
use crate::proxy::server::tls::tls_acceptor;
use crate::util::Result;

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

            let server = TcpListenerStream::new(TcpListener::bind(addr).await?)
                .try_for_each(|client_conn| async {
                    let acceptor = acceptor.to_owned();
                    let pod_api = pod_api.to_owned();
                    let pod_name = pod_name.to_string();
                    let pod_port = k8s_client.pod_port;

                    tokio::spawn(async move {
                        match handle_connection(client_conn, acceptor, pod_api, pod_name.to_string(), pod_port).await {
                            Ok(_) => {}
                            Err(e) => error!("Error on handle client, err: {:?}", e)
                        };
                    });

                    // keep the server running
                    Ok(())
                });

            info!("Proxy Server Initialized");

            server.await?;

            Ok(())
        } else {
            Err(format!("Pods in namespace {} with label {} not found", k8s_client.pod_namespace, k8s_client.pod_label).into())
        };
    }
}

async fn handle_client(
    pod_api: Api<Pod>,
    pod_name: String,
    pod_port: u16,
    client_conn: impl AsyncRead + AsyncWrite + Unpin + Send,
) -> Result<()> {
    let pods = pod_api.to_owned();

    return forward_connection(&pods, pod_name.as_str(), pod_port, client_conn).await;
}

async fn handle_connection(client_conn: TcpStream,
                           acceptor: Option<TlsAcceptor>,
                           pod_api: Api<Pod>,
                           pod_name: String,
                           pod_port: u16,
) -> Result<()> {
    match acceptor {
        None => handle_client(pod_api, pod_name, pod_port, client_conn).await?,
        Some(acc) => {
            let stream = acc.accept(client_conn).await?;
            handle_client(pod_api, pod_name, pod_port, stream).await?
        }
    };

    return Ok(());
}

async fn forward_connection(
    pods: &Api<Pod>,
    pod_name: &str,
    port: u16,
    mut client_conn: impl AsyncRead + AsyncWrite + Unpin + Send,
) -> Result<()> {
    let mut forwarder = pods.portforward(pod_name, &[port]).await?;
    let mut upstream_conn = forwarder.take_stream(port)
        .ok_or("Cannot get stream from port forward")?;

    // tokio::io::copy_bidirectional(&mut client_conn, &mut upstream_conn).await?;
    tokio::io::copy_bidirectional(&mut upstream_conn, &mut client_conn).await?;

    drop(upstream_conn);
    forwarder.join().await?;
    Ok(())
}