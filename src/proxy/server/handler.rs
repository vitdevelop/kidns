use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use futures::TryStreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::{Api, ResourceExt};
use log::error;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpListener;
use tokio_stream::wrappers::TcpListenerStream;
use crate::proxy::server::proxy::Proxy;
use crate::util::Result;

impl Proxy {
    pub async fn serve(self) -> Result<()> {
        let k8s_client = &self.k8s_client;

        let pod_list = k8s_client.pod_list().await?;
        let pod_api = k8s_client.pod_api()?;

        return if let Some(pod) = pod_list.items.first() {
            let addr = SocketAddr::from((Ipv4Addr::from_str(&self.host)?, self.port));

            let server = TcpListenerStream::new(TcpListener::bind(addr).await?)
                .try_for_each(|client_conn| async {
                    let pods = pod_api.clone();

                    let name = pod.name_any();
                    let pod_port = 80;
                    tokio::spawn(async move {
                        if let Err(e) = forward_connection(&pods, name.as_str(), pod_port, client_conn).await {
                            error!("Err forward connection: {:?}", e)
                        }
                    });
                    // keep the server running
                    Ok(())
                });
            server.await?;

            Ok(())
        } else {
            Err(format!("Pods in namespace {} with label {} not found", k8s_client.namespace, k8s_client.pod_label).into())
        };
    }
}

async fn forward_connection(
    pods: &Api<Pod>,
    pod_name: &str,
    port: u16,
    mut client_conn: impl AsyncRead + AsyncWrite + Unpin,
) -> Result<()> {
    let mut forwarder = pods.portforward(pod_name, &[port]).await?;
    let mut upstream_conn = forwarder.take_stream(port)
        .ok_or("Cannot get stream from port forward")?;

    tokio::io::copy_bidirectional(&mut client_conn, &mut upstream_conn).await?;

    drop(upstream_conn);
    forwarder.join().await?;
    Ok(())
}