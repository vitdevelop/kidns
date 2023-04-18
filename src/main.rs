use std::fs::File;
use std::io::Read;
use std::net::UdpSocket;
use crate::dns::buffer::BytePacketBuffer;
use crate::dns::header::QueryType;
use crate::dns::packet::DnsPacket;
use crate::dns::question::DnsQuestion;

mod util;
mod dns;

fn main() -> util::Result<()> {
    lookup("google.com", QueryType::MX)
}


fn lookup(qname: &str, qtype: QueryType) -> util::Result<()> {
    let sever = ("8.8.8.8", 53);

    let socket = UdpSocket::bind(("0.0.0.0", 43210))?;

    let mut packet = DnsPacket::new();

    packet.header.id = 6666;
    packet.header.questions = 1;
    packet.header.recursion_desired = true;
    packet.questions
        .push(DnsQuestion::new(qname.to_string(), qtype));

    let mut req_buffer = BytePacketBuffer::new();
    packet.write(&mut req_buffer)?;

    socket.send_to(&req_buffer.buf[0..req_buffer.pos], sever)?;

    let mut res_buffer = BytePacketBuffer::new();
    socket.recv_from(&mut res_buffer.buf)?;

    let res_packet = DnsPacket::from_buffer(&mut res_buffer)?;
    println!("{:#?}", res_packet.header);

    for q in res_packet.questions {
        println!("{:#?}", q);
    }

    for rec in res_packet.answers {
        println!("{:#?}", rec);
    }

    for rec in res_packet.authorities {
        println!("{:#?}", rec);
    }

    for rec in res_packet.resources {
        println!("{:#?}", rec);
    }

    return Ok(());
}

fn read_file() -> util::Result<()> {
    let mut file = File::open("testspace/response_packet.txt")?;
    let mut buffer = BytePacketBuffer::new();
    file.read(&mut buffer.buf)?;

    let packet = DnsPacket::from_buffer(&mut buffer)?;
    println!("{:#?}", packet.header);

    for q in packet.questions {
        println!("{:#?}", q);
    }

    for rec in packet.answers {
        println!("{:#?}", rec);
    }

    for rec in packet.authorities {
        println!("{:#?}", rec);
    }

    for rec in packet.resources {
        println!("{:#?}", rec);
    }

    return Ok(());
}
