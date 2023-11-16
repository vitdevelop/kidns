use crate::config::properties::Properties;
use crate::dns::record::DnsRecord;
use crate::k8s::client::K8sClient;
use crate::util::Result;
use chrono::{DateTime, Duration, Utc};
use log::info;
use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::ops::Add;
use std::path::Path;
use std::str::FromStr;
use std::sync::Arc;
use std::vec;
use tokio::fs::File;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct CacheRecord {
    pub records: Vec<DnsRecord>,
    pub expires: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Cache {
    pub domains: Arc<RwLock<HashMap<String, CacheRecord>>>,
}

impl Cache {
    pub async fn new(props: &Properties) -> Result<Cache> {
        let k8s_clients = {
            let mut clients = Vec::default();
            for props in &props.k8s {
                let client = K8sClient::new(props).await?;
                clients.push(client);
            }
            clients
        };

        let mut cache: HashMap<String, CacheRecord> = HashMap::new();

        for cache_type in &props.dns.cache {
            if cache_type.eq_ignore_ascii_case("k8s") {
                let k8s_cache = load_k8s_ingress_cache(&k8s_clients).await?;
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

    pub async fn find(&self, domain: &str) -> Option<CacheRecord> {
        let record = match self.domains.read().await.get(domain) {
            None => return None,
            Some(record) => record.to_owned(),
        };

        if record.expires < Utc::now() {
            self.domains.write().await.remove(domain);
            return None;
        }
        return Some(record);
    }
}

async fn load_k8s_ingress_cache(k8s_clients: &Vec<K8sClient>) -> Result<HashMap<String, CacheRecord>> {
    let mut urls = Vec::default();
    for client in k8s_clients {
        urls.extend(client.ingress_urls().await?);
    }
    return Ok(
        urls
        .iter()
        .map(|host| {
            return (
                host.to_string(),
                CacheRecord {
                    expires: Utc::now().add(Duration::days(365)),
                    records: vec![DnsRecord::A {
                        domain: host.to_owned(),
                        addr: Ipv4Addr::new(127, 0, 0, 1),
                        ttl: 300u32,
                    }],
                },
            );
        })
        .inspect(|host| info!("Ingress: {}", host.0))
        .collect());
}

async fn load_local_cache(path: &String) -> Result<HashMap<String, CacheRecord>> {
    let mut lines = read_lines(path).await?;

    let lines: HashMap<String, CacheRecord> = lines
        .iter_mut()
        .map(|line| line.split_once("="))
        .filter(|value| value.is_some())
        .map(|value| value.unwrap())
        .map(|(url, ip)| (url, Ipv4Addr::from_str(ip)))
        .filter(|(_, ip)| ip.is_ok())
        .map(|(url, ip)| (url, ip.unwrap()))
        .map(|(url, ip)| {
            return (
                url.to_string(),
                CacheRecord {
                    expires: Utc::now().add(Duration::days(365)),
                    records: vec![DnsRecord::A {
                        domain: url.to_string(),
                        addr: ip,
                        ttl: 300u32,
                    }],
                },
            );
        })
        .collect();

    return Ok(lines);
}

async fn read_lines<P>(filename: P) -> Result<Vec<String>>
where
    P: AsRef<Path>,
{
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
