use crate::util::Result;

pub struct BytePacketBuffer {
    pub buf: [u8; 512],
    pub pos: usize,
}

impl BytePacketBuffer {
    pub fn new() -> BytePacketBuffer {
        return BytePacketBuffer {
            buf: [0; 512],
            pos: 0,
        };
    }

    pub fn pos(&self) -> usize {
        return self.pos;
    }

    pub fn step(&mut self, steps: usize) -> Result<()> {
        self.pos += steps;

        return Ok(());
    }

    pub fn seek(&mut self, pos: usize) -> Result<()> {
        self.pos = pos;

        return Ok(());
    }

    pub fn read(&mut self) -> Result<u8> {
        self.validate_buffer_position()?;

        let response = self.buf[self.pos];
        self.pos += 1;

        return Ok(response);
    }

    pub fn get(&self, pos: usize) -> Result<u8> {
        self.validate_buffer_position()?;

        return Ok(self.buf[pos]);
    }

    pub fn get_range(&self, start: usize, len: usize) -> Result<&[u8]> {
        self.validate_buffer_position()?;

        return Ok(&self.buf[start..start + len]);
    }

    pub fn read_u16(&mut self) -> Result<u16> {
        return Ok(
            ((self.read()? as u16) << 8) | (self.read()? as u16)
        );
    }

    pub fn read_u32(&mut self) -> Result<u32> {
        return Ok(
            ((self.read()? as u32) << 24)
                | ((self.read()? as u32) << 16)
                | ((self.read()? as u32) << 8)
                | (self.read()? as u32)
        );
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

            let len = self.get(pos)?;

            if (len & 0xC0) == 0xC0 {
                if !jumped {
                    self.seek(pos + 2)?;
                }

                let b2 = self.get(pos + 1)? as u16;
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

                let str_buffer = self.get_range(pos, len as usize)?;
                outstr.push_str(&String::from_utf8_lossy(str_buffer).to_lowercase());

                delimit = ".";

                pos += len as usize;
            }
        }

        if !jumped {
            self.seek(pos)?;
        }

        return Ok(());
    }

    fn write(&mut self, val: u8) -> Result<()> {
        self.validate_buffer_position()?;

        self.buf[self.pos] = val;
        self.pos += 1;

        return Ok(());
    }

    pub fn write_u8(&mut self, val: u8) -> Result<()> {
        self.write(val)?;

        return Ok(());
    }

    pub fn write_u16(&mut self, val: u16) -> Result<()> {
        self.write((val >> 8) as u8)?;
        self.write((val & 0xFF) as u8)?;

        return Ok(());
    }

    pub fn write_u32(&mut self, val: u32) -> Result<()> {
        self.write((val >> 24) as u8)?;
        self.write((val >> 16) as u8)?;
        self.write((val >> 8) as u8)?;
        self.write((val & 0xFF) as u8)?;

        return Ok(());
    }

    pub fn write_qname(&mut self, qname: &str) -> Result<()> {
        for label in qname.split('.') {
            let len = label.len();
            if len > 0x3F { // 63
                return Err("Single label exceeds 63 characters of length".into());
            }

            self.write_u8(len as u8)?;
            for b in label.as_bytes() {
                self.write_u8(*b)?;
            }
        }

        self.write_u8(0)?;

        return Ok(());
    }

    pub fn set(&mut self, pos: usize, val: u8) -> Result<()> {
        self.buf[pos] = val;

        return Ok(());
    }

    pub fn set_u16(&mut self, pos: usize, val: u16) -> Result<()> {
        self.set(pos, (val >> 8) as u8)?;
        self.set(pos, (val & 0xFF) as u8)?;

        return Ok(());
    }

    fn validate_buffer_position(&self) -> Result<()> {
        if self.pos >= 512 {
            return Err("End of buffer".into());
        }

        return Ok(());
    }
}