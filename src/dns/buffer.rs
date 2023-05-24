use crate::util::Result;

pub const PACKET_SIZE: usize = 508; // 576 IPv4 (every host must be able to reassemble) - 60 IPv4 header - 8 UDP header
// pub const ADDITIONAL_PACKET_SIZE: usize = 1432; // default MTU(on modern routers) 1500 - 60 IPv4 header - 8 UDP header
pub struct BytePacketBuffer {
    pub buf: [u8; PACKET_SIZE],
    pub pos: usize,
    pub max_size: usize,
}

impl BytePacketBuffer {
    pub fn new() -> BytePacketBuffer {
        return BytePacketBuffer {
            buf: [0; PACKET_SIZE],
            pos: 0,
            max_size: PACKET_SIZE,
        };
    }

    pub fn pos(&self) -> usize {
        return self.pos;
    }

    pub fn step(&mut self, steps: usize) {
        self.pos += steps;
    }

    pub fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }

    pub fn read(&mut self) -> u8 {
        let response = self.buf[self.pos];
        self.pos += 1;

        return response;
    }

    pub fn get(&self, pos: usize) -> u8 {
        return self.buf[pos];
    }

    pub fn get_range(&self, start: usize, len: usize) -> &[u8] {
        return &self.buf[start..start + len];
    }

    pub fn read_u16(&mut self) -> u16 {
        return ((self.read() as u16) << 8) | (self.read() as u16);
    }

    pub fn read_u32(&mut self) -> u32 {
        return ((self.read() as u32) << 24)
            | ((self.read() as u32) << 16)
            | ((self.read() as u32) << 8)
            | (self.read() as u32);
    }

    pub fn read_qname(&mut self, outstr: &mut String) -> Result<()> {
        let mut pos = self.pos;

        let mut jumped = false;
        let max_jumps = 5;
        let mut jumps_performed = 0;

        let mut delimit = "";
        loop {
            if jumps_performed > max_jumps {
                return Err(format!("Limit of {} jumps exceeded", max_jumps).into());
            }

            let len = self.get(pos);

            if (len & 0xC0) == 0xC0 {
                if !jumped {
                    self.seek(pos + 2);
                }

                let b2 = self.get(pos + 1) as u16;
                let offset = (((len as u16) ^ 0xC0) << 8) | b2;
                pos = offset as usize;

                jumped = true;
                jumps_performed += 1;

                continue;
            } else {
                pos += 1;

                if len == 0 {
                    break;
                }

                outstr.push_str(delimit);

                let str_buffer = self.get_range(pos, len as usize);
                outstr.push_str(&String::from_utf8_lossy(str_buffer).to_lowercase());

                delimit = ".";

                pos += len as usize;
            }
        }

        if !jumped {
            self.seek(pos);
        }

        return Ok(());
    }

    fn write(&mut self, val: u8) {
        if self.pos >= self.max_size {
            return;
        }
        self.buf[self.pos] = val;
        self.pos += 1;
    }

    pub fn write_u8(&mut self, val: u8) {
        self.write(val);
    }

    pub fn write_u16(&mut self, val: u16) {
        self.write((val >> 8) as u8);
        self.write((val & 0xFF) as u8);
    }

    pub fn write_u32(&mut self, val: u32) {
        self.write((val >> 24) as u8);
        self.write((val >> 16) as u8);
        self.write((val >> 8) as u8);
        self.write((val & 0xFF) as u8);
    }

    pub fn write_qname(&mut self, qname: &str) -> Result<usize> {
        let mut size = 0usize;
        for label in qname.split('.') {
            let len = label.len();
            if len > 0x3F {
                // 63
                return Err("Single label exceeds 63 characters of length".into());
            }

            self.write_u8(len as u8);
            size += 1;

            for b in label.as_bytes() {
                self.write_u8(*b);
                size += 1;
            }
        }

        self.write_u8(0);
        size += 1;

        return Ok(size);
    }

    pub fn set(&mut self, pos: usize, val: u8) {
        self.buf[pos] = val;
    }

    pub fn set_u16(&mut self, pos: usize, val: u16) {
        self.set(pos, (val >> 8) as u8);
        self.set(pos + 1, (val & 0xFF) as u8);
    }
}
