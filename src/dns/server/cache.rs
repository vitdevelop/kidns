use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use log::debug;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::RwLock;
use crate::config::properties::Properties;
use crate::dns::record::{DnsRecord, TransientTtl};
use crate::k8s::client::K8sClient;
use crate::util::Result;

#[derive(Debug, Clone)]
pub struct Cache {
    pub domains: Arc<RwLock<HashMap<String, DnsRecord>>>,
}

impl Cache {
    pub async fn new(props: &Properties) -> Result<Cache> {
        let k8s_client = K8sClient::new(props).await?;

        let mut cache: HashMap<String, DnsRecord> = HashMap::new();

        for cache_type in get_cache_types(&props.dns_cache) {
            if cache_type.eq_ignore_ascii_case("k8s") {
                let k8s_cache = load_k8s_ingress_cache(&k8s_client).await?;
                cache = cache.into_iter().chain(k8s_cache).collect();
            } else {
                let file_cache = load_local_cache(&cache_type).await?;
                cache = cache.into_iter().chain(file_cache).collect();
            }
        }


        return Ok(Cache {
            domains: Arc::new(RwLock::new(cache)),
        });
    }
}

// if dns is set as default, need init cache after dns is up, otherwise k8s client won't reach api
async fn load_k8s_ingress_cache(k8s_client: &K8sClient) -> Result<HashMap<String, DnsRecord>> {
    return Ok(k8s_client.ingress_list().await?
        .iter_mut()
        .map(|ingress| ingress.spec.to_owned())
        .filter(|ingress| ingress.is_some()).map(|ingress| ingress.unwrap())
        .filter(|spec| spec.rules.is_some()).flat_map(|spec| spec.rules.unwrap())
        .filter(|rule| rule.host.is_some()).map(|rule| rule.host.unwrap())
        .map(|host| {
            return (host.to_string(), DnsRecord::A {
                domain: host,
                addr: Ipv4Addr::new(127, 0, 0, 1),
                ttl: TransientTtl(300),
            });
        })
        .inspect(|host| debug!("Ingress: {}", host.0))
        .collect());
}

async fn load_local_cache(path: &String) -> Result<HashMap<String, DnsRecord>> {
    let mut lines = read_lines(path).await?;

    let lines: HashMap<String, DnsRecord> = lines
        .iter_mut()
        .map(|line| line.split_once("="))
        .filter(|value| value.is_some()).map(|value| value.unwrap())
        .map(|(url, ip)| (url, Ipv4Addr::from_str(ip)))
        .filter(|(_, ip)| ip.is_ok())
        .map(|(url, ip)| (url, ip.unwrap()))
        .map(|(url, ip)| {
            return (url.to_string(), DnsRecord::A {
                domain: url.to_string(),
                addr: ip,
                ttl: TransientTtl(300),
            });
        }).collect();

    return Ok(lines);
}

async fn read_lines<P>(filename: P) -> Result<Vec<String>>
    where P: AsRef<Path>, {
    let file = match File::open(filename).await {
        Ok(f) => Ok(f),
        Err(e) => Err(format!("Can't open file, err: {:#?}", e)),
    }?;
    let mut line_buf = BufReader::new(file).lines();
    let mut lines: Vec<String> = Vec::default();

    while let Some(line) = line_buf.next_line().await? {
        lines.push(line);
    }

    return Ok(lines);
}

fn get_cache_types(types: &String) -> Vec<String> {
    return types.split(",")
        .map(|a| a.to_string())
        .collect();
}