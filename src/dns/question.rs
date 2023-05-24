use crate::dns::buffer::BytePacketBuffer;
use crate::dns::header::QueryType;
use crate::util::Result;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DnsQuestion {
    pub name: String,
    pub qtype: QueryType,
}

impl DnsQuestion {
    pub fn new(name: String, qtype: QueryType) -> DnsQuestion {
        return DnsQuestion {
            name,
            qtype,
        };
    }

    pub fn read(&mut self, buffer: &mut BytePacketBuffer) -> Result<()> {
        buffer.read_qname(&mut self.name)?;
        self.qtype = QueryType::from_num(buffer.read_u16());
        let _ = buffer.read_u16(); // class

        return Ok(());
    }

    pub fn write(&self, buffer: &mut BytePacketBuffer) -> Result<usize> {
        let mut size = 0usize;
        size += buffer.write_qname(&self.name)?;

        let typenum = self.qtype.to_num();
        buffer.write_u16(typenum);
        buffer.write_u16(1);

        size += 4;

        return Ok(size);
    }
}