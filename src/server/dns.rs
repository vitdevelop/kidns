use std::sync::Arc;
use tokio::net::UdpSocket;
use crate::config::properties::Properties;
use crate::dns::buffer::BytePacketBuffer;
use crate::util::Result;

#[derive(Debug, Clone)]
pub struct DnsServer {
    pub(in crate::server) public_dns_server: String,
    pub(in crate::server) host: String,
    pub(in crate::server) port: u16,
}

impl DnsServer {
    pub fn new(props: &Properties) -> Result<DnsServer> {
        return Ok(DnsServer {
            public_dns_server: props.dns_server_public.clone(),
            host: props.dns_server_host.clone(),
            port: props.dns_server_port,
        });
    }

    pub async fn serve(self) -> Result<()> {
        let socket = Arc::new(UdpSocket::bind((self.host.as_str(), self.port)).await?);
        let server = Arc::new(self);
        loop {
            let mut req_buffer = BytePacketBuffer::new();
            let (_, src) = socket.recv_from(&mut req_buffer.buf).await?;
            let dns_socket = socket.clone();
            let dns_server = server.clone();

            tokio::spawn(async move {
                match dns_server.handle_query(req_buffer, &dns_socket, src).await {
                    Ok(_) => {}
                    Err(e) => eprintln!("An error occurred: {}", e),
                }
            });
        }
    }
}