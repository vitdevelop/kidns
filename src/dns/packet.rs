use crate::dns::buffer::BytePacketBuffer;
use crate::dns::header::{DnsHeader, QueryType};
use crate::dns::question::DnsQuestion;
use crate::dns::record::DnsRecord;
use anyhow::Result;

#[derive(Clone, Debug)]
pub struct DnsPacket {
    pub header: DnsHeader,
    pub questions: Vec<DnsQuestion>,
    pub answers: Vec<DnsRecord>,
    pub authorities: Vec<DnsRecord>,
    pub resources: Vec<DnsRecord>,
}

impl DnsPacket {
    pub fn new() -> DnsPacket {
        return DnsPacket {
            header: DnsHeader::new(),
            questions: Vec::new(),
            answers: Vec::new(),
            authorities: Vec::new(),
            resources: Vec::new(),
        };
    }

    pub fn from_buffer(buffer: &mut BytePacketBuffer) -> Result<DnsPacket> {
        let mut result = DnsPacket::new();
        result.header.read(buffer);

        for _ in 0..result.header.questions {
            let mut question = DnsQuestion::new("".to_string(), QueryType::UNKNOWN(0));
            question.read(buffer)?;
            result.questions.push(question);
        }

        for _ in 0..result.header.answers {
            let rec = DnsRecord::read(buffer)?;
            result.answers.push(rec);
        }

        for _ in 0..result.header.authoritative_entries {
            let rec = DnsRecord::read(buffer)?;
            result.authorities.push(rec);
        }

        for _ in 0..result.header.resource_entries {
            let rec = DnsRecord::read(buffer)?;
            result.resources.push(rec);
        }

        return Ok(result);
    }

    pub fn write(&mut self, buffer: &mut BytePacketBuffer) -> Result<()> {
        self.header.questions = self.questions.len() as u16;
        self.header.answers = self.answers.len() as u16;
        self.header.authoritative_entries = self.authorities.len() as u16;
        self.header.resource_entries = self.resources.len() as u16;

        let mut size = 0usize;

        size += self.header.write(buffer);

        for ref question in &self.questions {
            size += question.write(buffer)?;
        }

        let mut record_count = self.answers.len() + self.authorities.len() + self.resources.len();

        // reset header
        self.header.questions = 0;
        self.header.answers = 0;
        self.header.authoritative_entries = 0;
        self.header.resource_entries = 0;

        for (i, rec) in self
            .answers
            .iter()
            .chain(self.authorities.iter())
            .chain(self.resources.iter())
            .enumerate()
        {
            size += rec.write(buffer)?;
            if size >= buffer.max_size {
                record_count = i;
                // self.header.truncated_message = true;

                // error!("Caught buffer overflow for: {:?}", rec);
                break;
            } else if i < self.answers.len() {
                self.header.answers += 1;
            } else if i < self.answers.len() + self.authorities.len() {
                self.header.authoritative_entries += 1;
            } else {
                self.header.resource_entries += 1;
            }
        }

        // reset all data
        // buffer.buf.iter_mut().for_each(|x| *x = 0);
        // for i in &mut buffer.buf[..] { *i = 0 }
        buffer.buf.fill(0);
        buffer.pos = 0;

        self.header.questions = self.questions.len() as u16;

        self.header.write(buffer);

        for question in &self.questions {
            question.write(buffer)?;
        }

        for rec in self
            .answers
            .iter()
            .chain(self.authorities.iter())
            .chain(self.resources.iter())
            .take(record_count)
        {
            rec.write(buffer)?;
        }

        return Ok(());
    }
}
