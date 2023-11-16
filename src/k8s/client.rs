use crate::config::properties::K8sProps;
use crate::util::Result;
use k8s_openapi::api::core::v1::Pod;
use k8s_openapi::api::networking::v1::Ingress;
use kube::api::{ListParams, ObjectList};
use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::{Api, Config};

#[derive(Clone)]
pub struct K8sClient {
    pub pod_namespace: String,
    pub pod_label: String,
    pub pod_port: u16,
    pub ingress_namespace: String,
    client: Option<kube::Client>,
}

impl K8sClient {
    pub async fn new(props: &K8sProps) -> Result<K8sClient> {
        let client = if props.config.eq_ignore_ascii_case("default") {
            kube::Client::try_default().await?
        } else {
            let yaml = tokio::fs::read_to_string(&props.config).await?;
            let kube_config = Kubeconfig::from_yaml(&yaml)?;
            let kube_options = KubeConfigOptions::default();
            let config = Config::from_custom_kubeconfig(kube_config, &kube_options).await?;

            kube::Client::try_from(config)?
        };

        return Ok(K8sClient {
            pod_namespace: props.pod.namespace.to_string(),
            pod_label: props.pod.label.to_string(),
            pod_port: props.pod.port,
            ingress_namespace: props.ingress_namespace.to_string(),
            client: Some(client),
        });
    }

    pub fn pod_api(&self) -> Result<Api<Pod>> {
        let client = self
            .client
            .to_owned()
            .ok_or("K8s client didn't initialized")?;
        return Ok(Api::namespaced(client, &self.pod_namespace));
    }

    pub async fn pod_list(&self) -> Result<ObjectList<Pod>> {
        let params = ListParams::default().labels(&self.pod_label);
        let pod_api = self.pod_api()?;
        return Ok(pod_api.list(&params).await?);
    }

    async fn ingress_api(&self) -> Result<Api<Ingress>> {
        let client = self
            .client
            .to_owned()
            .ok_or("K8s client didn't initialized")?;
        return Ok(Api::namespaced(client, &self.ingress_namespace));
    }

    pub async fn ingress_list(&self) -> Result<ObjectList<Ingress>> {
        let params = ListParams::default();
        let ingress_api = self.ingress_api().await?;
        return Ok(ingress_api.list(&params).await?);
    }

    pub async fn ingress_urls(&self) -> Result<Vec<String>> {
        return Ok(self
            .ingress_list()
            .await?
            .iter_mut()
            .map(|ingress| ingress.spec.to_owned())
            .filter(|ingress| ingress.is_some())
            .map(|ingress| ingress.unwrap())
            .filter(|spec| spec.rules.is_some())
            .flat_map(|spec| spec.rules.unwrap())
            .filter(|rule| rule.host.is_some())
            .map(|rule| rule.host.unwrap())
            .collect());
    }
}
