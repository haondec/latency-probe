// Placeholder for helper functions, e.g. host/ip resolution, parsing, etc.

use std::net::IpAddr;
use anyhow::Result;

pub fn parse_host_port(s: &str, default_port: u16) -> (String, u16) {
    if let Some(idx) = s.rfind(':') {
        if let Ok(port) = s[idx+1..].parse::<u16>() {
            return (s[..idx].to_string(), port);
        }
    }
    (s.to_string(), default_port)
}

pub async fn resolve_host_to_ip(host: &str) -> Result<IpAddr> {
    // First try to parse as IP address
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Ok(ip);
    }
    
    // If parsing fails, resolve via DNS
    let addr = format!("{}:0", host);
    let mut addrs = tokio::net::lookup_host(&addr).await?;
    Ok(addrs
        .next()
        .ok_or_else(|| anyhow::anyhow!("Could not resolve hostname: {}", host))?
        .ip())
}
