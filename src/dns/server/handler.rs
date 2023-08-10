use crate::dns::buffer::BytePacketBuffer;
use crate::dns::header::ResultCode::NOERROR;
use crate::dns::header::ResultCode;
use crate::dns::packet::DnsPacket;
use crate::dns::server::dns::DnsServer;
use crate::util::Result;
use log::{debug, warn};
use std::net::SocketAddr;
use tokio::net::UdpSocket;

impl DnsServer {
    pub async fn handle_query(
        &self,
        mut req_buffer: BytePacketBuffer,
        server_socket: &UdpSocket,
        client_socket: SocketAddr,
    ) -> Result<()> {
        let request = DnsPacket::from_buffer(&mut req_buffer)?;

        let mut packet = DnsPacket::new();
        packet.header.id = request.header.id;
        packet.header.recursion_desired = true;
        packet.header.recursion_available = true;
        packet.header.response = true;

        if let Some(question) = request.questions.last() {
            debug!("Received query: {:?}", question);

            let question_name = question.name.to_string();

            if let Some(dns_record) = self.cache.find(&question.name).await {
                packet.questions.push(question.to_owned());
                packet.header.rescode = NOERROR;
                packet.answers = dns_record.records;
            } else if let Ok(result) = self.lookup(request).await {
                if result.header.truncated_message {
                    warn!("Request to {} was truncated", question_name)
                }

                packet = result;
            } else {
                packet.header.rescode = ResultCode::SERVFAIL;
            }

        } else {
            packet.header.rescode = ResultCode::FORMERR;
        }

        let mut res_buffer = BytePacketBuffer::new();
        packet.write(&mut res_buffer)?;

        let len = res_buffer.pos();
        let data = res_buffer.get_range(0, len);

        server_socket.send_to(data, client_socket).await?;

        return Ok(());
    }

    pub async fn lookup(&self, mut packet: DnsPacket) -> Result<DnsPacket> {
        let server = (self.public_dns_server.as_str(), 53);

        let socket = UdpSocket::bind(("0.0.0.0", 0)).await?;

        packet.header.resource_entries = 0;
        packet.resources.clear();

        let mut req_buffer = BytePacketBuffer::new();
        packet.write(&mut req_buffer)?;

        socket
            .send_to(&req_buffer.buf[0..req_buffer.pos], server)
            .await?;

        let mut res_buffer = BytePacketBuffer::new();
        socket.recv_from(&mut res_buffer.buf).await?;

        return DnsPacket::from_buffer(&mut res_buffer);
    }
}
