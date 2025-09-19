use anyhow::Result;
use tokio::net::UdpSocket;
use tokio::time::{timeout, Duration, Instant};

pub async fn probe_echo(host: &str, port: u16) -> Result<Duration> {
    let addr = format!("{}:{}", host, port);
    let socket = UdpSocket::bind("0.0.0.0:0").await?;
    socket.connect(&addr).await?;
    let start = Instant::now();
    let msg = b"ping";
    socket.send(msg).await?;
    let mut buf = [0u8; 32];
    let recv_fut = socket.recv(&mut buf);
    timeout(Duration::from_millis(1000), recv_fut).await??;
    let elapsed = start.elapsed();
    Ok(elapsed)
}
