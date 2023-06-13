use crate::dns::buffer::BytePacketBuffer;
use crate::dns::header::ResultCode::NOERROR;
use crate::dns::header::{QueryType, ResultCode};
use crate::dns::packet::DnsPacket;
use crate::dns::question::DnsQuestion;
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
        let mut request = DnsPacket::from_buffer(&mut req_buffer)?;

        let mut packet = DnsPacket::new();
        packet.header.id = request.header.id;
        packet.header.recursion_desired = true;
        packet.header.recursion_available = true;
        packet.header.response = true;

        if let Some(question) = request.questions.pop() {
            debug!("Received query: {:?}", question);

            let question_name = question.name.to_string();
            let question_type = question.qtype;
            let mut need_cache = false;

            if let Some(dns_record) = self.cache.find(&question.name).await {
                packet.questions.push(question.to_owned());
                packet.header.rescode = NOERROR;
                packet.answers = dns_record.records;
            } else if let Ok(result) = self.lookup(question_name.as_str(), question_type).await {
                if result.header.truncated_message {
                    warn!("Request to {} was truncated", question_name)
                }
                need_cache = true;

                packet.questions.push(question);
                packet.header.rescode = result.header.rescode;

                for rec in result.answers {
                    debug!("Answer: {:#?}", rec);
                    packet.answers.push(rec);
                }
                for rec in result.authorities {
                    debug!("Authority: {:#?}", rec);
                    packet.authorities.push(rec);
                }
                for rec in result.resources {
                    debug!("Resource: {:#?}", rec);
                    packet.resources.push(rec);
                }
            } else {
                packet.header.rescode = ResultCode::SERVFAIL;
            }

            // save in cache
            if need_cache && packet.answers.len() > 0 {
                if let Some(question) = packet.questions.first() {
                    self.cache
                        .save(question.name.as_str(), &packet.answers)
                        .await?;
                }
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

    pub async fn lookup(&self, qname: &str, qtype: QueryType) -> Result<DnsPacket> {
        let server = (self.public_dns_server.as_str(), 53);

        let socket = UdpSocket::bind(("0.0.0.0", 0)).await?;

        let mut packet = DnsPacket::new();

        packet.header.id = 1234;
        packet.header.questions = 1;
        packet.header.recursion_desired = true;
        packet
            .questions
            .push(DnsQuestion::new(qname.to_string(), qtype));

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
