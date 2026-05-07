## Ipmark
markup IPs in stdin with geo-locations


### Usage Examples

echo 1.1.1.1 | target/release/ipmark -m       
1.1.1.1(AS13335 Cloudflare, Inc. Australia)

echo 1.1.1.1 | target/release/ipmark   
1.1.1.1(澳大利亚 APNIC/CloudFlare公共DNS服务器)

echo 2001:470:0:1f9::2 | target/release/ipmark 
2001:470:0:1f9::2(美国 California州 Fremont Hurricane Electric LLC 骨干网 - Fremont - IDC 机房 (bgp.he.net))

echo 2001:470:0:1f9::2 | target/release/ipmark -m
2001:470:0:1f9::2(AS6939 Hurricane Electric LLC United States)

