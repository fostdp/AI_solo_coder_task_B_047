use prometheus::{
    Encoder, IntCounter, IntCounterVec, HistogramOpts, HistogramVec,
    Opts, Registry, TextEncoder,
};
use std::sync::OnceLock;

static REGISTRY: OnceLock<Registry> = OnceLock::new();
static REQUEST_COUNTER: OnceLock<IntCounterVec> = OnceLock::new();
static REQUEST_DURATION: OnceLock<HistogramVec> = OnceLock::new();
static SYNTAX_COMPUTE_COUNTER: OnceLock<IntCounter> = OnceLock::new();
static FRACTAL_COMPUTE_COUNTER: OnceLock<IntCounter> = OnceLock::new();
static MK_TEST_COUNTER: OnceLock<IntCounter> = OnceLock::new();
static DB_QUERY_COUNTER: OnceLock<IntCounter> = OnceLock::new();

pub fn init_metrics() {
    let registry = Registry::new_custom(Some("ancient_city".to_string()), None)
        .expect("Failed to create prometheus registry");

    let request_counter = IntCounterVec::new(
        Opts::new("http_requests_total", "Total HTTP requests"),
        &["method", "endpoint", "status"]
    ).expect("Failed to create request counter");
    registry.register(Box::new(request_counter.clone())).expect("register failed");

    let request_duration = HistogramVec::new(
        HistogramOpts::new("http_request_duration_seconds", "HTTP request duration in seconds")
            .buckets(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]),
        &["method", "endpoint"]
    ).expect("Failed to create request duration histogram");
    registry.register(Box::new(request_duration.clone())).expect("register failed");

    let syntax_counter = IntCounter::new(
        "syntax_compute_total", "Total spatial syntax computations"
    ).expect("Failed to create syntax counter");
    registry.register(Box::new(syntax_counter.clone())).expect("register failed");

    let fractal_counter = IntCounter::new(
        "fractal_compute_total", "Total fractal dimension computations"
    ).expect("Failed to create fractal counter");
    registry.register(Box::new(fractal_counter.clone())).expect("register failed");

    let mk_counter = IntCounter::new(
        "mk_test_total", "Total Mann-Kendall tests executed"
    ).expect("Failed to create mk counter");
    registry.register(Box::new(mk_counter.clone())).expect("register failed");

    let db_counter = IntCounter::new(
        "db_query_total", "Total database queries"
    ).expect("Failed to create db counter");
    registry.register(Box::new(db_counter.clone())).expect("register failed");

    REGISTRY.set(registry).ok();
    REQUEST_COUNTER.set(request_counter).ok();
    REQUEST_DURATION.set(request_duration).ok();
    SYNTAX_COMPUTE_COUNTER.set(syntax_counter).ok();
    FRACTAL_COMPUTE_COUNTER.set(fractal_counter).ok();
    MK_TEST_COUNTER.set(mk_counter).ok();
    DB_QUERY_COUNTER.set(db_counter).ok();
}

pub fn registry() -> &'static Registry {
    REGISTRY.get().expect("Metrics not initialized")
}

pub fn record_request(method: &str, endpoint: &str, status: u16) {
    if let Some(c) = REQUEST_COUNTER.get() {
        c.with_label_values(&[method, endpoint, &status.to_string()]).inc();
    }
}

pub fn record_request_duration(method: &str, endpoint: &str, duration_secs: f64) {
    if let Some(h) = REQUEST_DURATION.get() {
        h.with_label_values(&[method, endpoint]).observe(duration_secs);
    }
}

pub fn inc_syntax_compute() {
    if let Some(c) = SYNTAX_COMPUTE_COUNTER.get() { c.inc(); }
}

pub fn inc_fractal_compute() {
    if let Some(c) = FRACTAL_COMPUTE_COUNTER.get() { c.inc(); }
}

pub fn inc_mk_test() {
    if let Some(c) = MK_TEST_COUNTER.get() { c.inc(); }
}

pub fn inc_db_query() {
    if let Some(c) = DB_QUERY_COUNTER.get() { c.inc(); }
}

pub fn gather_metrics_text() -> String {
    let encoder = TextEncoder::new();
    let metric_families = registry().gather();
    let mut buffer = Vec::new();
    encoder.encode(&metric_families, &mut buffer).unwrap_or_default();
    String::from_utf8(buffer).unwrap_or_default()
}
