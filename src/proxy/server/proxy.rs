use crate::util::Result;
use crate::config::properties::Properties;
use crate::k8s::client::K8sClient;

pub struct Proxy {
    pub(super) host: String,
    pub(super) port: u16,
    pub(super) k8s_client: K8sClient,
    pub(super) key_path: String,
    pub(super) cert_path: String,
}

impl Proxy {
    pub async fn new(props: &Properties) -> Result<Proxy> {
        let k8s_client = K8sClient::new(&props).await?;

        return Ok(Proxy {
            host: props.proxy_host.to_string(),
            port: props.proxy_port,
            k8s_client,
            key_path: props.proxy_tls_key.to_string(),
            cert_path: props.proxy_tls_cert.to_string(),
        });
    }
}
