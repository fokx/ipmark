mod qqwry_parser;

use lru::LruCache;
use maxminddb::Reader;
use maxminddb::geoip2;
use regex::Regex;
use std::io::{self, BufRead, Write};
use std::net::IpAddr;
use std::str::FromStr;

use crate::qqwry_parser::QQWryParser;
use ipaddress::IPAddress;
use std::num::NonZeroUsize;

/// Lookup IP info using MaxMind + QQWry
fn lookup_ip(
    ip: &str,
    qqwry: &mut QQWryParser,
    mmdb_city: &Reader<Vec<u8>>,
    mmdb_asn: &Reader<Vec<u8>>,
) -> String {
    let mut result = String::new();

    if let Ok(ip_addr) = IpAddr::from_str(ip) {
        if ip_addr.is_ipv4() {
            // 使用我们的 lookup 接口
            if let Some(loc) = qqwry.lookup(ip) {
                // 处理 QQWry 特有的占位符
                let country = loc.country;
                let area = loc.area;
                if !country.is_empty() || !area.is_empty() {
                    result.push_str(&format!("{} {}", country, area));
                }
            }
            result = result.replace(" CZ88.NET", "");
        } else {
            // MaxMind ASN
            if let Ok(asn) = mmdb_asn.lookup::<geoip2::Asn>(ip_addr) {
                let asn_number = asn.autonomous_system_number.unwrap_or(0);
                let asn_org = asn.autonomous_system_organization.unwrap_or("");
                result.push_str(&format!("AS{} {} ", asn_number, asn_org));
            }

            // MaxMind City
            if let Ok(city) = mmdb_city.lookup::<geoip2::City>(ip_addr) {
                let country = city
                    .country
                    .as_ref()
                    .and_then(|c| c.names.as_ref())
                    .and_then(|n| n.get("en"))
                    .map(|s| s.as_ref())
                    .unwrap_or("");
                let city_name = city
                    .city
                    .as_ref()
                    .and_then(|c| c.names.as_ref())
                    .and_then(|n| n.get("en"))
                    .map(|s| s.as_ref())
                    .unwrap_or("");
                if !country.is_empty() || !city_name.is_empty() {
                    result.push_str(&format!("{} {} ", country, city_name));
                }
            }
        }
    }

    result.trim().replace(['\n', '\t'], " ")
}

/// Transform a line by marking all IPs
fn transform_line(
    line: &str,
    ip_regex: &Regex,
    qqwry: &mut QQWryParser,
    mmdb_city: &Reader<Vec<u8>>,
    mmdb_asn: &Reader<Vec<u8>>,
    cache: &mut LruCache<String, String>,
) -> String {
    let mut shift = 0;
    let mut output = line.to_string();

    for mat in ip_regex.find_iter(line) {
        let ip_str = mat.as_str();
        if let Ok(ip_addr) = IPAddress::parse(ip_str) {
            if ip_addr.is_private() || ip_addr.is_loopback() {
                continue;
            }
        } else {
            continue;
        }

        // Cache repeated IPs
        let info = if let Some(cached) = cache.get(ip_str) {
            cached.clone()
        } else {
            let info = lookup_ip(ip_str, qqwry, mmdb_city, mmdb_asn);
            cache.put(ip_str.to_string(), info.clone());
            info
        };

        if !info.is_empty() {
            let pos = mat.end() + shift;
            output.insert_str(pos, &format!("({})", info));
            // output.insert_str(pos, &format!(",{}", info));
            shift += info.len() + 1; // account for the comma
        }
    }

    output
}

fn main() -> io::Result<()> {
    // IPv4 & IPv6 regex
    let ip_regex = Regex::new(
        r"((25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)|([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}|(([0-9a-fA-F]{1,4}:){0,6}[0-9a-fA-F]{1,4})?::(([0-9a-fA-F]{1,4}:){0,6}[0-9a-fA-F]{1,4})?"
    ).unwrap();
    let mut qqwry = crate::qqwry_parser::QQWryParser::new("/i/assets/ipmark/QQWry.Dat")
        .expect("Failed to load QQWry.Dat");
    // Load databases
    let mmdb_city =
        Reader::open_readfile("/usr/share/opensearch/modules/ingest-geoip/GeoLite2-City.mmdb")
            .expect("Failed to load GeoLite2-City.mmdb");
    let mmdb_asn =
        Reader::open_readfile("/usr/share/opensearch/modules/ingest-geoip/GeoLite2-ASN.mmdb")
            .expect("Failed to load GeoLite2-ASN.mmdb");

    // LRU cache for repeated IPs
    let mut cache = LruCache::new(NonZeroUsize::new(1000).unwrap());

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut handle_out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line?;
        let transformed = transform_line(
            &line, &ip_regex, &mut qqwry, &mmdb_city, &mmdb_asn, &mut cache,
        );
        writeln!(handle_out, "{}", transformed)?;
    }

    Ok(())
}
