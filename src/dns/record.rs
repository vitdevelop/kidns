use std::net::{Ipv4Addr, Ipv6Addr};
use crate::dns::buffer::BytePacketBuffer;
use crate::dns::header::QueryType;
use crate::util::Result;


#[derive(Copy, Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct TransientTtl(pub u32);

#[derive(Debug, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
#[allow(dead_code)]
pub enum DnsRecord {
    UNKNOWN {
        domain: String,
        qtype: u16,
        data_len: u16,
        ttl: TransientTtl,
    },
    // 0
    A {
        domain: String,
        addr: Ipv4Addr,
        ttl: TransientTtl,
    },
    // 1
    NS {
        domain: String,
        host: String,
        ttl: TransientTtl,
    },
    // 2
    CNAME {
        domain: String,
        host: String,
        ttl: TransientTtl,
    },
    // 5
    SOA {
        domain: String,
        m_name: String,
        r_name: String,
        serial: u32,
        refresh: u32,
        retry: u32,
        expire: u32,
        minimum: u32,
        ttl: TransientTtl,
    },
    // 6
    MX {
        domain: String,
        priority: u16,
        host: String,
        ttl: TransientTtl,
    },
    // 15
    TXT {
        domain: String,
        data: String,
        ttl: TransientTtl,
    },
    // 16
    AAAA {
        domain: String,
        addr: Ipv6Addr,
        ttl: TransientTtl,
    },
    // 28
    SRV {
        domain: String,
        priority: u16,
        weight: u16,
        port: u16,
        host: String,
        ttl: TransientTtl,
    },
    // 33
    OPT {
        packet_len: u16,
        flags: u32,
        data: String,
    }, // 41
}

impl DnsRecord {
    pub fn read(buffer: &mut BytePacketBuffer) -> Result<DnsRecord> {
        let mut domain = String::new();
        buffer.read_qname(&mut domain)?;

        let qtype_num = buffer.read_u16()?;
        let qtype = QueryType::from_num(qtype_num);
        let class = buffer.read_u16()?;
        let ttl = buffer.read_u32()?;
        let data_len = buffer.read_u16()?;

        return match qtype {
            QueryType::UNKNOWN(_) => {
                buffer.step(data_len as usize)?;

                Ok(DnsRecord::UNKNOWN {
                    domain,
                    qtype: qtype_num,
                    data_len,
                    ttl: TransientTtl(ttl),
                })
            }
            QueryType::A => {
                let raw_addr = buffer.read_u32()?;
                let addr = Ipv4Addr::new(
                    ((raw_addr >> 24) & 0xFF) as u8,
                    ((raw_addr >> 16) & 0xFF) as u8,
                    ((raw_addr >> 8) & 0xFF) as u8,
                    ((raw_addr) & 0xFF) as u8,
                );
                Ok(DnsRecord::A {
                    domain,
                    addr,
                    ttl: TransientTtl(ttl),
                })
            }
            QueryType::NS => {
                let mut ns = String::new();
                buffer.read_qname(&mut ns)?;

                Ok(DnsRecord::NS {
                    domain,
                    host: ns,
                    ttl: TransientTtl(ttl),
                })
            }
            QueryType::CNAME => {
                let mut cname = String::new();
                buffer.read_qname(&mut cname)?;

                Ok(DnsRecord::CNAME {
                    domain,
                    host: cname,
                    ttl: TransientTtl(ttl),
                })
            }
            QueryType::MX => {
                let priority = buffer.read_u16()?;
                let mut mx = String::new();
                buffer.read_qname(&mut mx)?;

                Ok(DnsRecord::MX {
                    domain,
                    priority,
                    host: mx,
                    ttl: TransientTtl(ttl),
                })
            }
            QueryType::AAAA => {
                let raw_addr1 = buffer.read_u32()?;
                let raw_addr2 = buffer.read_u32()?;
                let raw_addr3 = buffer.read_u32()?;
                let raw_addr4 = buffer.read_u32()?;

                let addr = Ipv6Addr::new(
                    ((raw_addr1 >> 16) & 0xFFFF) as u16,
                    (raw_addr1 & 0xFFFF) as u16,
                    ((raw_addr2 >> 16) & 0xFFFF) as u16,
                    (raw_addr2 & 0xFFFF) as u16,
                    ((raw_addr3 >> 16) & 0xFFFF) as u16,
                    (raw_addr3 & 0xFFFF) as u16,
                    ((raw_addr4 >> 16) & 0xFFFF) as u16,
                    (raw_addr4 & 0xFFFF) as u16,
                );

                Ok(DnsRecord::AAAA {
                    domain,
                    addr,
                    ttl: TransientTtl(ttl),
                })
            }
            QueryType::SOA => {
                let mut m_name = String::new();
                buffer.read_qname(&mut m_name)?;

                let mut r_name = String::new();
                buffer.read_qname(&mut r_name)?;

                let serial = buffer.read_u32()?;
                let refresh = buffer.read_u32()?;
                let retry = buffer.read_u32()?;
                let expire = buffer.read_u32()?;
                let minimum = buffer.read_u32()?;

                Ok(DnsRecord::SOA {
                    domain,
                    m_name,
                    r_name,
                    serial,
                    refresh,
                    retry,
                    expire,
                    minimum,
                    ttl: TransientTtl(ttl),
                })
            }
            QueryType::TXT => {
                let mut txt = String::new();

                let cur_pos = buffer.pos();
                txt.push_str(&String::from_utf8_lossy(
                    buffer.get_range(cur_pos, data_len as usize)?,
                ));

                buffer.step(data_len as usize)?;

                Ok(DnsRecord::TXT {
                    domain,
                    data: txt,
                    ttl: TransientTtl(ttl),
                })
            }
            QueryType::SRV => {
                let priority = buffer.read_u16()?;
                let weight = buffer.read_u16()?;
                let port = buffer.read_u16()?;

                let mut srv = String::new();
                buffer.read_qname(&mut srv)?;

                Ok(DnsRecord::SRV {
                    domain,
                    priority,
                    weight,
                    port,
                    host: srv,
                    ttl: TransientTtl(ttl),
                })
            }
            QueryType::OPT => {
                let mut data = String::new();

                let cur_pos = buffer.pos();
                data.push_str(&String::from_utf8_lossy(
                    buffer.get_range(cur_pos, data_len as usize)?,
                ));
                buffer.step(data_len as usize)?;

                Ok(DnsRecord::OPT {
                    packet_len: class,
                    flags: ttl,
                    data,
                })
            }
        };
    }

    pub fn write(&self, buffer: &mut BytePacketBuffer) -> Result<usize> {
        let start_pos = buffer.pos();

        match *self {
            DnsRecord::UNKNOWN { .. } => {
                println!("Skipping record: {:?}", self);
            }
            DnsRecord::A {
                ref domain,
                ref addr,
                ttl: TransientTtl(ttl),
            } => {
                buffer.write_qname(domain)?;
                buffer.write_u16(QueryType::A.to_num())?;
                buffer.write_u16(1)?;
                buffer.write_u32(ttl)?;
                buffer.write_u16(4)?;

                let octets = addr.octets();
                buffer.write_u8(octets[0])?;
                buffer.write_u8(octets[1])?;
                buffer.write_u8(octets[2])?;
                buffer.write_u8(octets[3])?;
            }
            DnsRecord::NS {
                ref domain,
                ref host,
                ttl: TransientTtl(ttl),
            } => {
                buffer.write_qname(domain)?;
                buffer.write_u16(QueryType::NS.to_num())?;
                buffer.write_u16(1)?;
                buffer.write_u32(ttl)?;

                let pos = buffer.pos();
                buffer.write_u16(0)?;

                buffer.write_qname(host)?;

                let size = buffer.pos() - (pos + 2);
                buffer.set_u16(pos, size as u16)?;
            }
            DnsRecord::CNAME {
                ref domain,
                ref host,
                ttl: TransientTtl(ttl),
            } => {
                buffer.write_qname(domain)?;
                buffer.write_u16(QueryType::CNAME.to_num())?;
                buffer.write_u16(1)?;
                buffer.write_u32(ttl)?;

                let pos = buffer.pos();
                buffer.write_u16(0)?;

                buffer.write_qname(host)?;

                let size = buffer.pos() - (pos + 2);
                buffer.set_u16(pos, size as u16)?;
            }
            DnsRecord::MX {
                ref domain,
                priority,
                ref host,
                ttl: TransientTtl(ttl),
            } => {
                buffer.write_qname(domain)?;
                buffer.write_u16(QueryType::MX.to_num())?;
                buffer.write_u16(1)?;
                buffer.write_u32(ttl)?;

                let pos = buffer.pos();
                buffer.write_u16(0)?;

                buffer.write_u16(priority)?;
                buffer.write_qname(host)?;

                let size = buffer.pos() - (pos + 2);
                buffer.set_u16(pos, size as u16)?;
            }
            DnsRecord::AAAA {
                ref domain,
                ref addr,
                ttl: TransientTtl(ttl),
            } => {
                buffer.write_qname(domain)?;
                buffer.write_u16(QueryType::AAAA.to_num())?;
                buffer.write_u16(1)?;
                buffer.write_u32(ttl)?;
                buffer.write_u16(16)?;

                for octet in &addr.segments() {
                    buffer.write_u16(*octet)?;
                }
            }
            DnsRecord::SOA {
                ref domain,
                ref m_name,
                ref r_name,
                serial,
                refresh,
                retry,
                expire,
                minimum,
                ttl: TransientTtl(ttl),
            } => {
                buffer.write_qname(domain)?;
                buffer.write_u16(QueryType::SOA.to_num())?;
                buffer.write_u16(1)?;
                buffer.write_u32(ttl)?;

                let pos = buffer.pos();
                buffer.write_u16(0)?;

                buffer.write_qname(m_name)?;
                buffer.write_qname(r_name)?;
                buffer.write_u32(serial)?;
                buffer.write_u32(refresh)?;
                buffer.write_u32(retry)?;
                buffer.write_u32(expire)?;
                buffer.write_u32(minimum)?;

                let size = buffer.pos() - (pos + 2);
                buffer.set_u16(pos, size as u16)?;
            }
            DnsRecord::TXT {
                ref domain,
                ref data,
                ttl: TransientTtl(ttl),
            } => {
                buffer.write_qname(domain)?;
                buffer.write_u16(QueryType::TXT.to_num())?;
                buffer.write_u16(1)?;
                buffer.write_u32(ttl)?;
                buffer.write_u16(data.len() as u16)?;

                for b in data.as_bytes() {
                    buffer.write_u8(*b)?;
                }
            }
            DnsRecord::SRV {
                ref domain,
                priority,
                weight,
                port,
                ref host,
                ttl: TransientTtl(ttl),
            } => {
                buffer.write_qname(domain)?;
                buffer.write_u16(QueryType::SRV.to_num())?;
                buffer.write_u16(1)?;
                buffer.write_u32(ttl)?;

                let pos = buffer.pos();
                buffer.write_u16(0)?;

                buffer.write_u16(priority)?;
                buffer.write_u16(weight)?;
                buffer.write_u16(port)?;
                buffer.write_qname(host)?;

                let size = buffer.pos() - (pos + 2);
                buffer.set_u16(pos, size as u16)?;
            }
            DnsRecord::OPT { .. } => {}
        }

        return Ok(buffer.pos() - start_pos);
    }
}