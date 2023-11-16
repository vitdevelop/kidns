use crate::util::Result;
use serde::Deserialize;
use std::fs::File;
use std::string::ToString;

fn empty() -> String {
    "".to_string()
}
fn google_dns() -> String {
    "8.8.8.8".to_string()
}
const fn port_53() -> u16 {
    53
}
const fn port_80() -> u16 {
    80
}
fn default() -> String {
    "default".to_string()
}
fn ingress_label() -> String {
    "app.kubernetes.io/name=ingress".to_string()
}

#[derive(Deserialize)]
pub struct Properties {
    pub dns: DnsProps,
    pub k8s: Vec<K8sProps>,
    pub proxy: ProxyProps,

    #[serde(rename = "log-level")]
    pub log_level: String,
}

#[derive(Deserialize)]
pub struct DnsProps {
    pub server: DnsServerProps,
    pub cache: Vec<String>,
}

#[derive(Deserialize)]
pub struct DnsServerProps {
    #[serde(default = "google_dns")]
    pub public: String,

    #[serde(default = "port_53")]
    pub port: u16,

    #[serde(default = "empty")]
    pub host: String,
}

#[derive(Deserialize)]
pub struct K8sProps {
    #[serde(rename = "ingress-namespace", default = "default")]
    pub ingress_namespace: String,

    pub pod: K8sPodProps,

    #[serde(default = "default")]
    pub config: String,
}

#[derive(Deserialize)]
pub struct K8sPodProps {
    #[serde(default = "default")]
    pub namespace: String,

    #[serde(default = "ingress_label")]
    pub label: String,

    #[serde(default = "port_80")]
    pub port: u16,
}

#[derive(Deserialize)]
pub struct ProxyProps {
    #[serde(default = "empty")]
    pub host: String,

    #[serde(default = "port_80")]
    pub port: u16,

    pub tls: ProxyTlsProps,
}

#[derive(Deserialize)]
pub struct ProxyTlsProps {
    #[serde(default = "empty")]
    pub cert: String,

    #[serde(default = "empty")]
    pub key: String,
}

pub fn parse_properties() -> Result<Properties> {
    let config_file = File::open("config.yaml")?;
    let config = serde_yaml::from_reader::<File, Properties>(config_file)?;
    return Ok(config);
}