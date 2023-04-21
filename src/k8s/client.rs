use k8s_openapi::api::core::v1::Pod;
use k8s_openapi::api::networking::v1::Ingress;
use kube::{Api, Config};
use kube::api::{ListParams, ObjectList};
use kube::client::ConfigExt;
use kube::config::{Kubeconfig, KubeConfigOptions};
use tower::ServiceBuilder;
use crate::config::properties::Properties;
use crate::util::Result;

#[derive(Clone)]
pub struct K8sClient {
    pub pod_namespace: String,
    pub pod_label: String,
    pub pod_port: u16,
    pub ingress_namespace: String,
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

            let https = config.rustls_https_connector()?;
            let service = ServiceBuilder::new()
                .layer(config.base_uri_layer())
                .service(hyper::Client::builder().build(https));
            kube::Client::new(service, config.default_namespace)

            // kube::Client::try_from(config)?
        };

        return Ok(K8sClient {
            pod_namespace: props.k8s_pod_namespace.to_string(),
            pod_label: props.k8s_pod_label.to_string(),
            pod_port: props.k8s_pod_port,
            ingress_namespace: props.k8s_ingress_namespace.to_string(),
            client: Some(client),
        });
    }

    pub fn pod_api(&self) -> Result<Api<Pod>> {
        let client = self.client.to_owned().ok_or("K8s client didn't initialized")?;
        return Ok(Api::namespaced(client, &self.pod_namespace));
    }

    pub async fn pod_list(&self) -> Result<ObjectList<Pod>> {
        let params = ListParams::default().labels(&self.pod_label);
        let pod_api = self.pod_api()?;
        return Ok(pod_api.list(&params).await?);
    }

    async fn ingress_api(&self) -> Result<Api<Ingress>> {
        let client = self.client.to_owned().ok_or("K8s client didn't initialized")?;
        return Ok(Api::namespaced(client, &self.ingress_namespace));
    }

    pub async fn ingress_list(&self) -> Result<ObjectList<Ingress>> {
        let params = ListParams::default();
        let ingress_api = self.ingress_api().await?;
        return Ok(ingress_api.list(&params).await?);
    }
}