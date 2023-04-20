use crate::util::Result;
use crate::config::properties::Properties;
use crate::proxy::k8s::client::K8sClient;

pub struct Proxy {
    pub(super) host: String,
    pub(super) port: u16,
    pub(super) k8s_client: K8sClient,
}

impl Proxy {
    pub async fn new(props: &Properties) -> Result<Proxy> {
        let k8s_client = K8sClient::new(&props).await?;

        return Ok(Proxy {
            host: props.proxy_host.clone(),
            port: props.proxy_port,
            k8s_client,
        });
    }
}
