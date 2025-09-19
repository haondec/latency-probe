use anyhow::Result;
use reqwest::Client;
use tokio::time::{timeout, Duration, Instant};

pub async fn probe_http(url: &str) -> Result<Duration> {
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;
    let start = Instant::now();
    let resp_fut = client.get(url).send();
    let resp = timeout(Duration::from_secs(30), resp_fut).await??;
    // you might want to measure until headers / first byte etc.
    let _ = resp.text().await?;
    let elapsed = start.elapsed();
    Ok(elapsed)
}
