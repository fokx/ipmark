## Ipmark
markup IPs in stdin with geo-locations


### Usage
```sh
target/release/ipmark -h
Markup IPs in stdin or arguments with geo-locations

Usage: ipmark [OPTIONS] [INPUTS]...

Arguments:
  [INPUTS]...  Optional IP strings/text to parse. If empty, reads from stdin

Options:
  -m, --maxmind                      Use Maxmind instead of QQWry
      --qqwry <QQWRY>                [env: QQWRY=] [default: /i/assets/ipmark/QQWry.Dat]
      --ipv6wry <IPV6WRY>            [env: IPV6WRY=] [default: /i/assets/ipmark/ipv6wry.db]
      --maxmind-city <MAXMIND_CITY>  [env: MAXMIND_CITY=] [default: /usr/share/opensearch/modules/ingest-geoip/GeoLite2-City.mmdb]
      --maxmind-asn <MAXMIND_ASN>    [env: MAXMIND_ASN=] [default: /usr/share/opensearch/modules/ingest-geoip/GeoLite2-ASN.mmdb]
  -h, --help                         Print help
  -V, --version                      Print version
```

### Examples
```sh
echo 1.1.1.1 | target/release/ipmark -m       
1.1.1.1(AS13335 Cloudflare, Inc. Australia)

echo 1.1.1.1 | target/release/ipmark   
1.1.1.1(澳大利亚 APNIC/CloudFlare公共DNS服务器)

echo 2001:470:0:1f9::2 | target/release/ipmark -m
2001:470:0:1f9::2(AS6939 Hurricane Electric LLC United States)

echo 2001:470:0:1f9::2 | target/release/ipmark 
2001:470:0:1f9::2(美国 California州 Fremont Hurricane Electric LLC 骨干网 - Fremont - IDC 机房 (bgp.he.net))

target/release/ipmark '2001:470:0:1f9::2 1.1.1.1'
2001:470:0:1f9::2(美国 California州 Fremont Hurricane Electric LLC 骨干网 - Fremont - IDC机房 (bgp.he.net)) 1.1.1.1(澳大利亚 APNIC/CloudFlare公共DNS服务器)
```

