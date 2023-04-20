use k8s_openapi::api::core::v1::Pod;
use kube::{Api, Config};
use kube::api::{ListParams, ObjectList};
use kube::config::{Kubeconfig, KubeConfigOptions};
use crate::config::properties::Properties;
use crate::util::Result;

#[derive(Clone)]
pub struct K8sClient {
    pub namespace: String,
    pub pod_label: String,
    pub pod_port: u16,
    client: Option<kube::Client>,
}

impl K8sClient {
    pub async fn new(props: &Properties) -> Result<K8sClient> {
        let client = if props.k8s_config.eq_ignore_ascii_case("default") {
            kube::Client::try_default().await?
        } else {
            let yaml = tokio::fs::read_to_string(&props.k8s_config).await?;
            let kube_config = Kubeconfig::from_yaml(&yaml)?;
            let kube_options = KubeConfigOptions::default();
            let config = Config::from_custom_kubeconfig(kube_config, &kube_options).await?;

            kube::Client::try_from(config)?
        };

        return Ok(K8sClient {
            namespace: props.k8s_namespace.clone(),
            pod_label: props.k8s_pod_label.clone(),
            pod_port: props.k8s_pod_port,
            client: Some(client),
        });
    }

    pub fn pod_api(&self) -> Result<Api<Pod>> {
        let client = self.client.clone().ok_or("K8s client didn't initialized")?;
        return Ok(Api::namespaced(client, &self.namespace));
    }

    pub async fn pod_list(&self) -> Result<ObjectList<Pod>> {
        let params = ListParams::default().labels(&self.pod_label);
        let pod_api = self.pod_api()?;
        return Ok(pod_api.list(&params).await?);
    }
}