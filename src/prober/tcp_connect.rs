use anyhow::Result;
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration, Instant};

pub async fn probe_tcp(host: &str, port: u16) -> Result<Duration> {
    let addr = format!("{}:{}", host, port);
    let start = Instant::now();
    let conn_fut = TcpStream::connect(addr);
    let conn = timeout(Duration::from_millis(3000), conn_fut).await??;
    drop(conn);
    let elapsed = start.elapsed();
    Ok(elapsed)
}
