use crate::config::properties::K8sProps;
use crate::ingress_spec;
use crate::util::Result;
use k8s_openapi::api::core::v1::{Pod, Secret};
use k8s_openapi::api::networking::v1::Ingress;
use kube::api::{ListParams, ObjectList};
use kube::config::{KubeConfigOptions, Kubeconfig};
use kube::{Api, Config, ResourceExt};
use log::debug;
use tokio::io::{AsyncRead, AsyncWrite};

#[derive(Clone)]
pub struct K8sClient {
    pub pod_namespace: String,
    pub pod_label: String,
    pub pod_http_port: u16,
    pub pod_https_port: u16,
    pub ingress_namespace: String,
    client: Option<kube::Client>,
}

const TLS_KEY_SECRET: &str = "tls.key";
const TLS_CERT_SECRET: &str = "tls.crt";

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
            pod_http_port: props.pod.port.http,
            pod_https_port: props.pod.port.https,
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
        return Ok(ingress_spec!(self)
            .filter(|spec| spec.rules.is_some())
            .flat_map(|spec| spec.rules.unwrap())
            .filter(|rule| rule.host.is_some())
            .map(|rule| rule.host.unwrap())
            .collect());
    }

    async fn secrets_api(&self) -> Result<Api<Secret>> {
        let client = self
            .client
            .to_owned()
            .ok_or("K8s client didn't initialized")?;
        return Ok(Api::namespaced(client, &self.ingress_namespace));
    }

    pub async fn secrets_list(&self) -> Result<ObjectList<Secret>> {
        let params = ListParams::default();
        let secrets_api = self.secrets_api().await?;
        return Ok(secrets_api.list(&params).await?);
    }

    /// Return private key and cert
    pub async fn tls_cert(&self, server_name: &String) -> Result<(Vec<u8>, Vec<u8>)> {
        match ingress_spec!(self)
            .map(|ingress| ingress.tls)
            .map(|tls| tls.unwrap())
            .flat_map(|tls| tls)
            .filter(|tls| tls.hosts.is_some())
            .filter_map(|tls| {
                match tls
                    .hosts
                    .unwrap()
                    .iter()
                    .filter(|host| *host == server_name)
                    .next()
                {
                    None => None,
                    Some(_) => tls.secret_name,
                }
            })
            .next()
        {
            None => Err(format!("Can't found ingress {}", server_name).into()),
            Some(secret_name) => {
                let (key, cert) = self
                    .secrets_list()
                    .await?
                    .iter()
                    .filter(|secret| secret.metadata.name.is_some())
                    .filter(|secret| secret.metadata.name.as_ref().unwrap() == &secret_name)
                    .filter(|secret| secret.data.is_some())
                    .filter(|secret| {
                        secret.data.as_ref().unwrap().contains_key(TLS_KEY_SECRET)
                            && secret.data.as_ref().unwrap().contains_key(TLS_CERT_SECRET)
                    })
                    .map(|secret| {
                        let key = &secret.data.as_ref().unwrap().get(TLS_KEY_SECRET).unwrap().0;
                        let cert = &secret.data.as_ref().unwrap().get(TLS_CERT_SECRET).unwrap().0;
                        (
                            key.clone(),
                            cert.clone(),
                        )
                    })
                    .next()
                    .ok_or(format!("Unable to find cert for {}", server_name))?;

                Ok((key, cert))
            }
        }
    }

    pub async fn get_port_forwarder(
        &self,
        secure: bool,
    ) -> Result<impl AsyncRead + AsyncWrite + Unpin> {
        let mut pod_list = self.pod_list().await?;
        let pod_api = self.pod_api()?;

        let pod = pod_list.items.pop().ok_or("Unable to find free pod port")?;
        let pod_name = pod.name_any();
        debug!("Connect to {}", pod_name);

        let pod_port = if secure {
            self.pod_https_port
        } else {
            self.pod_http_port
        };

        let mut forwarder = pod_api.portforward(pod_name.as_str(), &[pod_port]).await?;
        let upstream_conn = forwarder
            .take_stream(pod_port)
            .ok_or("Cannot get stream from port forward")?;

        Ok(upstream_conn)
    }
}
