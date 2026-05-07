mod ipv6wry_parser;
mod qqwry_parser;

use lru::LruCache;
use maxminddb::Reader;
use maxminddb::geoip2;
use regex::Regex;
use std::io::{self, BufRead, Write};
use std::net::IpAddr;
use std::str::FromStr;

use crate::ipv6wry_parser::IPv6WryParser;
use crate::qqwry_parser::QQWryParser;
use ipaddress::IPAddress;
use std::num::NonZeroUsize;

use clap::Parser;

/// Markup IPs in stdin with geo-locations
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Use Maxmind instead of QQWry
    #[arg(short, long, default_value = "false")]
    maxmind: bool,
    #[arg(long, env = "QQWRY", default_value = "/i/assets/ipmark/QQWry.Dat")]
    qqwry: String,
    #[arg(long, env = "IPV6WRY", default_value = "/i/assets/ipmark/ipv6wry.db")]
    ipv6wry: String,
    #[arg(
        long,
        env = "MAXMIND_CITY",
        default_value = "/usr/share/opensearch/modules/ingest-geoip/GeoLite2-City.mmdb"
    )]
    maxmind_city: String,
    #[arg(
        long,
        env = "MAXMIND_ASN",
        default_value = "/usr/share/opensearch/modules/ingest-geoip/GeoLite2-ASN.mmdb"
    )]
    maxmind_asn: String,
}

struct Mmdb {
    enabled: bool,
    mmdb_city: Option<Reader<Vec<u8>>>,
    mmdb_asn: Option<Reader<Vec<u8>>>,
}

/// Lookup IP info using MaxMind + QQWry
fn lookup_ip(
    ip: &str,
    qqwry: &mut QQWryParser,
    ipv6wry: &IPv6WryParser,
    mmdb: &Mmdb,
    // mmdb_city: &Reader<Vec<u8>>,
    // mmdb_asn: &Reader<Vec<u8>>,
) -> String {
    let mut result = String::new();

    if let Ok(ip_addr) = IpAddr::from_str(ip) {
        if mmdb.enabled {
            if result.is_empty() {
                // MaxMind ASN
                if let Ok(asn) = mmdb
                    .mmdb_asn
                    .as_ref()
                    .unwrap()
                    .lookup::<geoip2::Asn>(ip_addr)
                {
                    let asn_number = asn.autonomous_system_number.unwrap_or(0);
                    let asn_org = asn.autonomous_system_organization.unwrap_or("");
                    result.push_str(&format!("AS{} {} ", asn_number, asn_org));
                }

                // MaxMind City
                if let Ok(city) = mmdb
                    .mmdb_city
                    .as_ref()
                    .unwrap()
                    .lookup::<geoip2::City>(ip_addr)
                {
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
        } else {
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
                // 使用新的 IPv6Wry
                if let Some(loc) = ipv6wry.lookup(ip) {
                    result.push_str(&loc);
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
    ipv6wry: &mut IPv6WryParser,
    mmdb: &Mmdb,
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
            let info = lookup_ip(ip_str, qqwry, ipv6wry, mmdb);
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
    let cli = Cli::parse();

    // IPv4 & IPv6 regex
    let ip_regex = Regex::new(
        r"((25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)|([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}|(([0-9a-fA-F]{1,4}:){0,6}[0-9a-fA-F]{1,4})?::(([0-9a-fA-F]{1,4}:){0,6}[0-9a-fA-F]{1,4})?"
    ).unwrap();
    let mut qqwry = QQWryParser::new(&cli.qqwry).expect("Failed to load QQWry.Dat");
    let mut ipv6wry = IPv6WryParser::new(&cli.ipv6wry).expect("Failed to load IPv6Wry.Dat");

    // LRU cache for repeated IPs
    let mut cache = LruCache::new(NonZeroUsize::new(1000).unwrap());

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut handle_out = stdout.lock();
    let mmdb = if cli.maxmind {
        let mmdb_city =
            Reader::open_readfile(&cli.maxmind_city).expect("Failed to load GeoLite2-City.mmdb");
        let mmdb_asn =
            Reader::open_readfile(&cli.maxmind_asn).expect("Failed to load GeoLite2-ASN.mmdb");

        Mmdb {
            enabled: cli.maxmind,
            mmdb_city: Some(mmdb_city),
            mmdb_asn: Some(mmdb_asn),
        }
    } else {
        Mmdb {
            enabled: cli.maxmind,
            mmdb_city: None,
            mmdb_asn: None,
        }
    };

    for line in stdin.lock().lines() {
        let line = line?;
        let transformed = transform_line(
            &line,
            &ip_regex,
            &mut qqwry,
            &mut ipv6wry,
            &mmdb,
            &mut cache,
        );
        writeln!(handle_out, "{}", transformed)?;
    }

    Ok(())
}
