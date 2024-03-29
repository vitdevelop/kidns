use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::{anyhow, Result};
use rustls::ServerConfig;
use tokio::sync::RwLock;

use crate::config::properties::Properties;
use crate::k8s::client::K8sClient;
use crate::proxy::server::cert::get_root_ca_params;
use crate::proxy::server::tls::CertificateData;
use crate::util::load_local_cache;

pub struct Proxy {
    pub(super) host: String,
    pub(super) http_port: u16,
    pub(super) https_port: u16,
    pub(super) k8s_clients: Vec<Arc<K8sClient>>,
    pub(super) ingress_clients: HashMap<String, Arc<K8sClient>>,
    pub(super) local_clients: HashMap<String, SocketAddr>,
    pub(super) destinations_certs: RwLock<HashMap<String, Arc<ServerConfig>>>,
    pub(super) root_cert: Option<CertificateData>,
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

        let local_clients_paths: Vec<String> = props
            .dns
            .cache
            .iter()
            .filter(|x| x.as_str().ne("k8s"))
            .map(|x| x.clone())
            .collect();

        let mut local_clients: HashMap<String, SocketAddr> = HashMap::new();
        for filename in local_clients_paths {
            let file_cache = load_local_cache(&filename).await?;
            local_clients = local_clients.into_iter().chain(file_cache).collect();
        }

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
            local_clients,
            destinations_certs: RwLock::new(HashMap::new()),
            root_cert: ca_certificate,
        });
    }
}