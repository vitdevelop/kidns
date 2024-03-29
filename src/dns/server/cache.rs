use crate::config::properties::Properties;
use crate::dns::record::DnsRecord;
use crate::k8s::client::K8sClient;
use anyhow::Result;
use log::info;
use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::ops::Add;
use std::sync::Arc;
use std::vec;
use time::{Duration, OffsetDateTime};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct CacheRecord {
    pub records: Vec<DnsRecord>,
    pub expires: OffsetDateTime,
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
                let file_cache = load_local_dns_cache(&cache_type).await?;
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

        if record.expires < OffsetDateTime::now_utc() {
            self.domains.write().await.remove(domain);
            return None;
        }
        return Some(record);
    }
}

async fn load_k8s_ingress_cache(
    k8s_clients: &Vec<K8sClient>,
) -> Result<HashMap<String, CacheRecord>> {
    let mut urls = Vec::default();
    for client in k8s_clients {
        urls.extend(client.ingress_urls().await?);
    }
    return Ok(urls
        .iter()
        .map(|host| {
            return (
                host.to_string(),
                CacheRecord {
                    expires: OffsetDateTime::now_utc().add(Duration::days(365)),
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

async fn load_local_dns_cache(path: &String) -> Result<HashMap<String, CacheRecord>> {
    let lines = crate::util::load_local_cache(path)
        .await?
        .iter()
        .map(|(url, ip)| {
            let dns_record = match ip {
                SocketAddr::V4(ip4) => DnsRecord::A {
                    domain: url.to_string(),
                    addr: ip4.ip().clone(),
                    ttl: 300u32,
                },
                SocketAddr::V6(ip6) => DnsRecord::AAAA {
                    domain: url.to_string(),
                    addr: ip6.ip().clone(),
                    ttl: 300u32,
                },
            };
            return (
                url.to_string(),
                CacheRecord {
                    expires: OffsetDateTime::now_utc().add(Duration::days(365)),
                    records: vec![dns_record],
                },
            );
        })
        .collect();

    return Ok(lines);
}
