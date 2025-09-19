use surge_ping::ping;
use std::time::Duration;
use anyhow::Result;
use crate::util::resolve_host_to_ip;

pub async fn probe_icmp(host: &str, _timeout_ms: u64) -> Result<Duration> {
    // Parse the host to IP address
    let ip_addr = resolve_host_to_ip(host).await?;
    
    // Create a simple payload - using process ID as identifier in the payload
    let process_id = std::process::id() as u16;
    let payload = process_id.to_be_bytes();
    
    // Send ping and measure time
    let (_packet, duration) = ping(ip_addr, &payload).await?;
    
    Ok(duration)
}