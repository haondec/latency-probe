use std::time::{Duration, SystemTime};
use libc::{clock_gettime, timespec, CLOCK_MONOTONIC_RAW};

pub fn monotonic_ns() -> u128 {
    unsafe {
        let mut ts: timespec = std::mem::zeroed();
        if clock_gettime(CLOCK_MONOTONIC_RAW, &mut ts) == 0 {
            (ts.tv_sec as u128) * 1_000_000_000 + (ts.tv_nsec as u128)
        } else {
            // fallback
            let now = SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_else(|_| Duration::from_secs(0));
            (now.as_secs() as u128) * 1_000_000_000 + (now.subsec_nanos() as u128)
        }
    }
}
