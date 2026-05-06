use regex::Regex;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::net::IpAddr;
use std::str::FromStr;
use maxminddb::Reader;
use qqwry::QQWry;

fn is_internal_ip(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let octets = v4.octets();
            (octets[0] == 10)
                || (octets[0] == 172 && (16..=31).contains(&octets[1]))
                || (octets[0] == 192 && octets[1] == 168)
                || v4.is_loopback()
        }
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local(),
    }
}

fn lookup_ip(
    ip: &str,
    qqwry: &QQWry,
    mmdb_city: &Reader<Vec<u8>>,
    mmdb_asn: &Reader<Vec<u8>>,
) -> String {
    let mut result = String::new();

    // MaxMind ASN
    if let Ok(ip_addr) = IpAddr::from_str(ip) {
        if let Ok(asn) = mmdb_asn.lookup::<maxminddb::geoip2::Asn>(ip_addr) {
            result.push_str(&format!(
                "AS{} {} ",
                asn.autonomous_system_number.unwrap_or(0),
                asn.autonomous_system_organization.unwrap_or("".into())
            ));
        }

        // MaxMind City
        if let Ok(city) = mmdb_city.lookup::<maxminddb::geoip2::City>(ip_addr) {
            let country = city
                .country
                .as_ref()
                .and_then(|c| c.names.as_ref())
                .and_then(|n| n.get("en"))
                .unwrap_or(&"".to_string());
            let city_name = city
                .city
                .as_ref()
                .and_then(|c| c.names.as_ref())
                .and_then(|n| n.get("en"))
                .unwrap_or(&"".to_string());
            result.push_str(&format!("{} {} ", country, city_name));
        }
    }

    // QQWry
    if let Ok(addr) = qqwry.query(ip) {
        result.push_str(&addr.to_string());
    }

    result.trim().replace('\n', " ").replace('\t', " ")
}

fn transform_line(
    line: &str,
    ip_regex: &Regex,
    qqwry: &QQWry,
    mmdb_city: &Reader<Vec<u8>>,
    mmdb_asn: &Reader<Vec<u8>>,
) -> String {
    let mut shift = 0;
    let mut output = line.to_string();

    for mat in ip_regex.find_iter(line) {
        let ip_str = mat.as_str();
        if let Ok(ip_addr) = IpAddr::from_str(ip_str) {
            if is_internal_ip(&ip_addr) {
                continue;
            }
        } else {
            continue;
        }

        let info = lookup_ip(ip_str, qqwry, mmdb_city, mmdb_asn);
        if !info.is_empty() {
            let pos = mat.end() + shift;
            output.insert_str(pos, &format!(",{}", info));
            shift += info.len() + 1; // account for the comma
        }
    }

    output.replace(" CZ88.NET", "")
}

fn main() -> io::Result<()> {
    let ip_regex = Regex::new(
        r"((25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)\.){3}(25[0-5]|2[0-4][0-9]|[01]?[0-9][0-9]?)|([0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}|(([0-9a-fA-F]{1,4}:){0,6}[0-9a-fA-F]{1,4})?::(([0-9a-fA-F]{1,4}:){0,6}[0-9a-fA-F]{1,4})?"
    ).unwrap();

    let qqwry = QQWry::new("qqwry.dat").unwrap();
    let mmdb_city = Reader::open_readfile("GeoLite2-City.mmdb").unwrap();
    let mmdb_asn = Reader::open_readfile("GeoLite2-ASN.mmdb").unwrap();

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut handle_out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = line?;
        let transformed = transform_line(&line, &ip_regex, &qqwry, &mmdb_city, &mmdb_asn);
        writeln!(handle_out, "{}", transformed)?;
    }

    Ok(())
}
