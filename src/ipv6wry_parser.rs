use byteorder::{ByteOrder, LittleEndian};
use std::fs::File;
use std::io::Read;
use std::net::IpAddr;

pub struct IPv6WryParser {
    data: Vec<u8>,
    index_base_offset: u64,
    count: u64,
    ip_version: u8,
    address_segment_len: u8,
}

impl IPv6WryParser {
    pub fn new(path: &str) -> std::io::Result<Self> {
        let mut file = File::open(path)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;

        if &data[0..4] != b"IPDB" {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Bad Magic",
            ));
        }

        let ip_version = data[7]; // 4 或 8 (代表 IPv4 或 IPv6)
        let count = LittleEndian::read_u64(&data[8..16]);
        let index_base_offset = LittleEndian::read_u64(&data[16..24]);

        // 如果 data[4] != 1，使用 data[24] 作为段长度，否则默认为 2
        let address_segment_len = if data[4] != 1 { data[24] } else { 2 };

        Ok(IPv6WryParser {
            data,
            index_base_offset,
            count,
            ip_version,
            address_segment_len,
        })
    }

    pub fn lookup(&self, ip_str: &str) -> Option<String> {
        let ip_addr: IpAddr = ip_str.parse().ok()?;

        // 提取用于搜索的 "Needle" (IPv4 是全值，IPv6 是前 64 位)
        let needle: u64 = match ip_addr {
            IpAddr::V4(v4) => {
                if self.ip_version != 4 {
                    return None;
                }
                u32::from(v4) as u64
            }
            IpAddr::V6(v6) => {
                if self.ip_version != 8 {
                    return None;
                }
                let octets = v6.octets();
                // 取前 8 字节作为 u64
                LittleEndian::read_u64(&[
                    octets[7], octets[6], octets[5], octets[4], octets[3], octets[2], octets[1],
                    octets[0],
                ])
            }
        };

        self.search_record(needle)
    }

    fn search_record(&self, needle: u64) -> Option<String> {
        let mut lo = 0;
        let mut hi = self.count - 1;
        let index_step = if self.ip_version == 4 { 7 } else { 11 };

        let mut hit_offset = self.index_base_offset;

        while lo <= hi {
            let mid = (lo + hi) / 2;
            let (mid_ip, offset) = self.read_index(mid, index_step);

            if needle >= mid_ip {
                hit_offset = offset;
                lo = mid + 1;
            } else {
                if mid == 0 {
                    break;
                }
                hi = mid - 1;
            }
        }

        Some(self.read_record(hit_offset as usize))
    }

    fn read_index(&self, i: u64, step: u32) -> (u64, u64) {
        let pos = (self.index_base_offset + i * step as u64) as usize;
        if self.ip_version == 4 {
            let ip = LittleEndian::read_u32(&self.data[pos..pos + 4]) as u64;
            let offset = self.read_u24(pos + 4) as u64;
            (ip, offset)
        } else {
            let ip = LittleEndian::read_u64(&self.data[pos..pos + 8]);
            let offset = self.read_u24(pos + 8) as u64;
            (ip, offset)
        }
    }

    fn read_record(&self, mut pos: usize) -> String {
        let mut parts = Vec::new();
        for _ in 0..self.address_segment_len {
            let mode = self.data[pos];
            if mode == 2 {
                let offset = self.read_u24(pos + 1) as usize;
                parts.push(self.read_cstring(offset));
                pos += 4;
            } else {
                let (s, next_pos) = self.read_cstring_with_next(pos);
                parts.push(s);
                pos = next_pos;
            }
        }
        parts.join(" ").trim().to_string()
    }

    fn read_cstring(&self, pos: usize) -> String {
        let (s, _) = self.read_cstring_with_next(pos);
        s
    }

    fn read_cstring_with_next(&self, start: usize) -> (String, usize) {
        if start == 0 {
            return ("".to_string(), 1);
        }
        let mut end = start;
        while end < self.data.len() && self.data[end] != 0 {
            end += 1;
        }
        // zxinc 的 IPDB 通常已经是 UTF-8，直接转换
        let s = String::from_utf8_lossy(&self.data[start..end]).into_owned();
        (s, end + 1)
    }

    fn read_u24(&self, pos: usize) -> u32 {
        (self.data[pos] as u32)
            | ((self.data[pos + 1] as u32) << 8)
            | ((self.data[pos + 2] as u32) << 16)
    }
}
