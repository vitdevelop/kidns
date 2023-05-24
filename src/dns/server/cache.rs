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
        let k8s_client = K8sClient::new(props).await?;

        let mut cache: HashMap<String, CacheRecord> = HashMap::new();

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

    pub async fn find(&self, domain: &str) -> Option<CacheRecord> {
        let record = match self.domains.read().await.get(domain) {
            None => return None,
            Some(record) => {
               record.to_owned()
            }
        };

        if record.expires < Utc::now() {
            self.domains.write().await.remove(domain);
            return None
        }
        return Some(record)
    }

    pub async fn save(&self, domain: &str, dns_records: &Vec<DnsRecord>) -> Result<()> {
        let expires = 'expire: {
            let mut biggest_ttl = 0u32;
            for record in dns_records {
                let ttl = match record {
                    DnsRecord::UNKNOWN { .. } => 0,
                    DnsRecord::A { ttl, .. } => *ttl,
                    DnsRecord::NS { ttl, .. } => *ttl,
                    DnsRecord::CNAME { ttl, .. } => *ttl,
                    DnsRecord::SOA { ttl, .. } => *ttl,
                    DnsRecord::MX { ttl, .. } => *ttl,
                    DnsRecord::PTR { ttl, .. } => *ttl,
                    DnsRecord::TXT { ttl, .. } => *ttl,
                    DnsRecord::AAAA { ttl, .. } => *ttl,
                    DnsRecord::SRV { ttl, .. } => *ttl,
                    DnsRecord::OPT { .. } => 0,
                };

                if biggest_ttl < ttl {
                    biggest_ttl = ttl;
                }
            }

            break 'expire Utc::now() + Duration::seconds(biggest_ttl as i64);
        };

        let mut domains = self.domains.write().await;
        domains.insert(
            domain.to_string(),
            CacheRecord {
                records: dns_records.to_owned(),
                expires,
            },
        );

        Ok(())
    }
}

// if dns is set as default, need init cache after dns is up, otherwise k8s client won't reach api
async fn load_k8s_ingress_cache(k8s_client: &K8sClient) -> Result<HashMap<String, CacheRecord>> {
    return Ok(k8s_client
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
        .map(|host| {
            return (
                host.to_string(),
                CacheRecord {
                    expires: Utc::now().add(Duration::days(365)),
                    records: vec![DnsRecord::A {
                        domain: host,
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

fn get_cache_types(types: &String) -> Vec<String> {
    return types.split(",").map(|a| a.to_string()).collect();
}
