use crate::config::properties::Properties;
use crate::k8s::client::K8sClient;
use crate::util::Result;
use std::collections::HashMap;
use std::sync::Arc;

pub struct Proxy {
    pub(super) host: String,
    pub(super) port: u16,
    pub(super) k8s_clients: Vec<Arc<K8sClient>>,
    pub(super) ingress_clients: HashMap<String, Arc<K8sClient>>,
    pub(super) key_path: String,
    pub(super) cert_path: String,
}

impl Proxy {
    pub async fn new(props: &Properties) -> Result<Proxy> {
        let mut ingress_clients = HashMap::<String, Arc<K8sClient>>::new();
        let mut k8s_clients = Vec::<Arc<K8sClient>>::with_capacity(props.k8s.len());
        for k8s_props in &props.k8s {
            let k8s_client = Arc::new(K8sClient::new(&k8s_props).await?);

            for url in k8s_client.ingress_urls().await? {
                ingress_clients.insert(url, k8s_client.clone());
            }

            k8s_clients.push(k8s_client.clone());
        }

        return Ok(Proxy {
            host: props.proxy.host.to_string(),
            port: props.proxy.port,
            k8s_clients,
            ingress_clients,
            key_path: props.proxy.tls.key.to_string(),
            cert_path: props.proxy.tls.cert.to_string(),
        });
    }
}
