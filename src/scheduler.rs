use std::time::Duration;
use tokio::time::{sleep_until, Instant};
use anyhow::Result;

pub struct Scheduler {
    interval: Duration,
}

impl Scheduler {
    pub fn new(interval_ms: u64) -> Result<Self> {
        Ok(Self {
            interval: Duration::from_millis(interval_ms),
        })
    }

    /// job: async closure for each tick
    pub async fn run<J, F>(&self, mut job: J) -> Result<()>
    where
        J: FnMut() -> F + Send + 'static,
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let mut next = Instant::now();
        loop {
            next += self.interval;
            // spawn job so next tick unaffected by job duration
            tokio::spawn(job());
            sleep_until(next).await;
        }
    }
}
