use crate::util::Result;

static DNS_SERVER_PUBLIC: &str = "DNS_SERVER_PUBLIC";
static DNS_SERVER_PORT: &str = "DNS_SERVER_PORT";
static DNS_SERVER_HOST: &str = "DNS_SERVER_HOST";
static K8S_NAMESPACE: &str = "K8S_NAMESPACE";
static K8S_POD_LABEL: &str = "K8S_POD_LABEL";
static K8S_POD_PORT: &str = "K8S_POD_PORT";
/// if set value 'default' will get config from KUBECONFIG or ~/.kube/config
/// otherwise read yaml file path
static K8S_CONFIG: &str = "K8S_CONFIG";
static PROXY_HOST: &str = "PROXY_HOST";
static PROXY_PORT: &str = "PROXY_PORT";
static PROXY_TLS_CERT: &str = "PROXY_TLS_CERT";
static PROXY_TLS_KEY: &str = "PROXY_TLS_KEY";
pub static LOG_LEVEL: &str = "LOG_LEVEL";

pub struct Properties {
    pub dns_server_public: String,
    pub dns_server_port: u16,
    pub dns_server_host: String,
    pub k8s_namespace: String,
    pub k8s_pod_label: String,
    pub k8s_pod_port: u16,
    pub k8s_config: String,
    pub proxy_host: String,
    pub proxy_port: u16,
    pub proxy_tls_cert: String,
    pub proxy_tls_key: String,
    pub log_level: String,
}

pub fn parse_properties() -> Result<Properties> {
    dotenv::from_filename("config.env").ok();

    return Ok(Properties {
        dns_server_public: get_optional_env_var(DNS_SERVER_PUBLIC, "8.8.8.8")?, // google dns
        dns_server_port: get_optional_env_var(DNS_SERVER_PORT, "53")?.parse()?,
        dns_server_host: get_optional_env_var(DNS_SERVER_HOST, "0.0.0.0")?,
        k8s_namespace: get_optional_env_var(K8S_NAMESPACE, "default")?,
        k8s_pod_label: get_optional_env_var(K8S_POD_LABEL, "app.kubernetes.io/name=ingress")?,
        k8s_pod_port: get_optional_env_var(K8S_POD_PORT, "80")?.parse()?,
        k8s_config: get_optional_env_var(K8S_CONFIG, "default")?,
        proxy_host: get_optional_env_var(PROXY_HOST, "0.0.0.0")?,
        proxy_port: get_optional_env_var(PROXY_PORT, "80")?.parse()?,
        proxy_tls_cert: get_optional_env_var(PROXY_TLS_CERT, "")?,
        proxy_tls_key: get_optional_env_var(PROXY_TLS_KEY, "")?,
        log_level: get_env_var(LOG_LEVEL)?,
    });
}

fn get_optional_env_var(var: &str, default: &str) -> Result<String> {
    return std::env::var(var)
        .or(Ok(default.to_string()));
}

fn get_env_var(var: &str) -> Result<String> {
    return std::env::var(var)
        .or(Err(format!("{} var not defined", var).into()));
}