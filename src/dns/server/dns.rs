use crate::config::properties::Properties;
use crate::dns::buffer::BytePacketBuffer;
use crate::dns::server::cache::Cache;
use anyhow::Result;
use log::info;
use std::sync::Arc;
use tokio::net::UdpSocket;

#[derive(Debug, Clone)]
pub struct DnsServer {
    pub(crate) public_dns_server: String,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) cache: Cache,
}

impl DnsServer {
    pub async fn new(props: &Properties) -> Result<DnsServer> {
        return Ok(DnsServer {
            public_dns_server: props.dns.server.public.to_string(),
            host: props.dns.server.host.to_string(),
            port: props.dns.server.port,
            cache: Cache::new(props).await?,
        });
    }

    pub async fn serve(self) -> Result<()> {
        let socket = Arc::new(UdpSocket::bind((self.host.as_str(), self.port)).await?);
        let server = Arc::new(self);

        info!("DNS Server Initialized");

        loop {
            let mut req_buffer = BytePacketBuffer::new();
            let (_, src) = socket.recv_from(&mut req_buffer.buf).await?;
            let dns_socket = socket.to_owned();
            let dns_server = server.to_owned();

            tokio::spawn(async move {
                match dns_server.handle_query(req_buffer, &dns_socket, src).await {
                    Ok(_) => {}
                    // Err(e) => error!("An error occurred: {}", e),
                    Err(_) => {} //suppress temporary errors
                }
            });
        }
    }
}
