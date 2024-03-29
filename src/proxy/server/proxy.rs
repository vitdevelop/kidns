use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use rcgen::Certificate;
use rustls::ServerConfig;
use tokio::sync::RwLock;

use crate::config::properties::Properties;
use crate::k8s::client::K8sClient;
use crate::proxy::server::cert::get_root_ca_params;

pub struct Proxy {
    pub(super) host: String,
    pub(super) http_port: u16,
    pub(super) https_port: u16,
    pub(super) k8s_clients: Vec<Arc<K8sClient>>,
    pub(super) ingress_clients: HashMap<String, Arc<K8sClient>>,
    pub(super) destinations_certs: RwLock<HashMap<String, DestinationConfig>>,
    pub(super) root_cert: Option<Certificate>,
}

impl Proxy {
    pub async fn new(props: &Properties) -> Result<Proxy> {
        let mut ingress_clients = HashMap::<String, Arc<K8sClient>>::new();
        let k8s_clients = match &props.k8s {
            Some(k8s_props) => {
                let mut k8s_clients = Vec::<Arc<K8sClient>>::with_capacity(k8s_props.len());
                for k8s_prop in k8s_props {
                    let k8s_client = Arc::new(K8sClient::new(k8s_prop).await?);

                    for url in k8s_client.ingress_urls().await? {
                        ingress_clients.insert(url, k8s_client.clone());
                    }

                    k8s_clients.push(k8s_client.clone());
                }
                k8s_clients
            }
            None => {
                vec![]
            }
        };
        let proxy_props = match &props.proxy {
            None => Err(anyhow!("Proxy properties is missing")),
            Some(proxy_props) => Ok(proxy_props),
        }?;

        let ca_certificate = match &proxy_props.root_ca {
            None => None,
            Some(tls_props) => Some(get_root_ca_params(&tls_props.key, &tls_props.cert).await?),
        };

        return Ok(Proxy {
            host: proxy_props.host.to_string(),
            http_port: proxy_props.port.http,
            https_port: proxy_props.port.https,
            k8s_clients,
            ingress_clients,
            destinations_certs: RwLock::new(HashMap::new()),
            root_cert: ca_certificate,
        });
    }
}

#[derive(Clone)]
pub struct DestinationConfig {
    pub server_config: Arc<ServerConfig>,
    pub k8s_client: Option<Arc<K8sClient>>,
}

impl DestinationConfig {
    pub fn new(
        server_config: Arc<ServerConfig>,
        k8s_client: Option<Arc<K8sClient>>,
    ) -> DestinationConfig {
        return DestinationConfig {
            server_config,
            k8s_client,
        };
    }
}
