use byteorder::{ByteOrder, LittleEndian, ReadBytesExt};
use encoding_rs::GBK;
use std::fs::File;
use std::io::Read;

pub struct QQWryParser {
    data: Vec<u8>,
    index_start: u32,
    index_end: u32,
}

#[derive(Debug, Clone)]
pub struct IpLocation {
    pub country: String,
    pub area: String,
}

impl QQWryParser {
    pub fn new(path: &str) -> std::io::Result<Self> {
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        let mut cursor = std::io::Cursor::new(&data);
        let index_start = cursor.read_u32::<LittleEndian>()?;
        let index_end = cursor.read_u32::<LittleEndian>()?;

        Ok(QQWryParser {
            data,
            index_start,
            index_end,
        })
    }

    pub fn lookup(&self, ip_str: &str) -> Option<IpLocation> {
        let ip = self.ip_to_u32(ip_str)?;
        let mut l = 0;
        let mut r = (self.index_end - self.index_start) / 7;

        while l <= r {
            let m = (l + r) / 2;
            let pos = (self.index_start + m * 7) as usize;
            let start_ip = LittleEndian::read_u32(&self.data[pos..pos + 4]);

            if ip < start_ip {
                r = m - 1;
            } else {
                let offset = self.read_u24(pos + 4) as usize;
                let end_ip = LittleEndian::read_u32(&self.data[offset..offset + 4]);
                if ip <= end_ip {
                    return Some(self.read_location(offset + 4));
                }
                l = m + 1;
            }
        }
        None
    }

    fn read_location(&self, offset: usize) -> IpLocation {
        let mode = self.data[offset];
        if mode == 0x01 {
            let country_offset = self.read_u24(offset + 1) as usize;
            self.read_location(country_offset)
        } else {
            let (country, next_offset) = self.read_area(offset);
            let (area, _) = self.read_area(next_offset);
            IpLocation { country, area }
        }
    }

    fn read_area(&self, offset: usize) -> (String, usize) {
        let mode = self.data[offset];
        if mode == 0x01 || mode == 0x02 {
            let off = self.read_u24(offset + 1) as usize;
            let (s, _) = self.read_string(off);
            (s, offset + 4)
        } else {
            self.read_string(offset)
        }
    }

    fn read_string(&self, offset: usize) -> (String, usize) {
        let mut end = offset;
        while self.data[end] != 0 {
            end += 1;
        }
        let gbk_bytes = &self.data[offset..end];
        let (res, _, _) = GBK.decode(gbk_bytes);
        (res.into_owned(), end + 1)
    }

    fn read_u24(&self, offset: usize) -> u32 {
        (self.data[offset] as u32)
            | ((self.data[offset + 1] as u32) << 8)
            | ((self.data[offset + 2] as u32) << 16)
    }

    fn ip_to_u32(&self, ip: &str) -> Option<u32> {
        let parts: Vec<u32> = ip.split('.').filter_map(|s| s.parse().ok()).collect();
        if parts.len() == 4 {
            Some((parts[0] << 24) | (parts[1] << 16) | (parts[2] << 8) | parts[3])
        } else {
            None
        }
    }
}
