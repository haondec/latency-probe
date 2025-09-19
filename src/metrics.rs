use prometheus::{Encoder, TextEncoder, HistogramVec, IntCounterVec, GaugeVec, Opts, Registry};
use warp::Filter;
use std::net::SocketAddr;
use once_cell::sync::Lazy;
use std::sync::Arc;

static REGISTRY: Lazy<Registry> = Lazy::new(|| Registry::new());

// Optional histogram for latency history - only registered if enabled
static LATENCY_HIST: Lazy<Option<HistogramVec>> = Lazy::new(|| None);

static LATENCY_GAUGE: Lazy<GaugeVec> = Lazy::new(|| {
    let opts = Opts::new("probe_latency_milliseconds_current", "Current probe latency in milliseconds");
    let gauge = GaugeVec::new(opts, &["target", "probe_type"]).unwrap();
    REGISTRY.register(Box::new(gauge.clone())).unwrap();
    gauge
});

static TIMEOUT_COUNTER: Lazy<IntCounterVec> = Lazy::new(|| {
    let opts = Opts::new("probe_timeout_total", "Total number of probe timeouts");
    let ctr = IntCounterVec::new(opts, &["target", "probe_type"]).unwrap();
    REGISTRY.register(Box::new(ctr.clone())).unwrap();
    ctr
});

// Track whether histogram is enabled
static mut HISTOGRAM_ENABLED: bool = false;
static HISTOGRAM_INSTANCE: Lazy<Arc<std::sync::Mutex<Option<HistogramVec>>>> = 
    Lazy::new(|| Arc::new(std::sync::Mutex::new(None)));

pub fn initialize_metrics(enable_latency_history: bool) {
    unsafe {
        HISTOGRAM_ENABLED = enable_latency_history;
    }
    
    if enable_latency_history {
        let opts = Opts::new("probe_latency_milliseconds", "Probe latency in milliseconds");
        let hist = HistogramVec::new(
            prometheus::HistogramOpts {
                common_opts: opts,
                buckets: vec![
                    0.05, 0.1, 0.2, 0.5, 1.0,
                    2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 250.0, 500.0, 1000.0
                ],
            },
            &["target", "probe_type"],
        ).expect("creating histogram");
        
        REGISTRY.register(Box::new(hist.clone())).unwrap();
        
        let mut guard = HISTOGRAM_INSTANCE.lock().unwrap();
        *guard = Some(hist);
    }
}

pub async fn serve_metrics(addr: SocketAddr) {
    let metrics_route = warp::path!("metrics").map(move || {
        let encoder = TextEncoder::new();
        let mf = REGISTRY.gather();
        let mut buf = Vec::new();
        encoder.encode(&mf, &mut buf).unwrap();
        warp::http::Response::builder()
            .header("Content-Type", encoder.format_type())
            .body(buf)
            .unwrap()
    });

    warp::serve(metrics_route).run(addr).await;
}

pub fn observe_latency(target: &str, probe_type: &str, latency_ms: f64) {
    // Always observe current latency in gauge
    LATENCY_GAUGE
        .with_label_values(&[target, probe_type])
        .set(latency_ms);
    
    // Conditionally observe latency history in histogram
    unsafe {
        if HISTOGRAM_ENABLED {
            if let Ok(guard) = HISTOGRAM_INSTANCE.lock() {
                if let Some(ref hist) = *guard {
                    hist.with_label_values(&[target, probe_type])
                        .observe(latency_ms);
                }
            }
        }
    }
}

pub fn inc_timeout(target: &str, probe_type: &str) {
    TIMEOUT_COUNTER
        .with_label_values(&[target, probe_type])
        .inc();
}

