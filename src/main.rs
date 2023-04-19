use std::fs::File;
use std::io::Read;
use crate::config::logs::init_logs;
use crate::config::properties::parse_properties;
use crate::dns::buffer::BytePacketBuffer;
use crate::dns::packet::DnsPacket;
use crate::server::dns::DnsServer;
use crate::util::Result;

mod util;
mod dns;
mod server;
mod config;


#[tokio::main]
async fn main() -> Result<()> {
    init_logs();
    let props = parse_properties()?;

    let dns = DnsServer::new(&props)?;
    return dns.serve().await
}

#[allow(dead_code)]
fn read_file() -> Result<()> {
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
