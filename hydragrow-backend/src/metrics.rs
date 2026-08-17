use lazy_static::lazy_static;
use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounter, IntCounterVec, IntGauge, Opts, Registry,
    TextEncoder,
};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    // 1. HTTP Metrics
    pub static ref HTTP_REQUESTS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("http_requests_total", "Tổng số HTTP requests"),
        &["method", "endpoint", "status"]
    ).expect("metric can be created");

    pub static ref HTTP_REQ_DURATION_SECONDS: HistogramVec = HistogramVec::new(
        HistogramOpts::new("http_request_duration_seconds", "Thời gian xử lý HTTP request tính bằng giây")
            .buckets(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]),
        &["method", "endpoint"]
    ).expect("metric can be created");

    // 2. WebSocket Metrics
    pub static ref ACTIVE_WS_CONNECTIONS: IntGauge = IntGauge::new(
        "active_ws_connections",
        "Số lượng kết nối WebSocket đang hoạt động"
    ).expect("metric can be created");

    // 3. MQTT Metrics
    pub static ref MQTT_MESSAGES_RECEIVED_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("mqtt_messages_received_total", "Tổng số message MQTT nhận được theo topic"),
        &["topic_suffix"]
    ).expect("metric can be created");

    pub static ref MQTT_PROCESSING_ERRORS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("mqtt_processing_errors_total", "Tổng số lỗi khi xử lý MQTT message"),
        &["topic_suffix", "error_type"]
    ).expect("metric can be created");

    // 4. Domain / IoT Metrics
    pub static ref SENSOR_UPDATES_TOTAL: IntCounter = IntCounter::new(
        "sensor_updates_total",
        "Tổng số bản ghi dữ liệu cảm biến đã nhận"
    ).expect("metric can be created");

    pub static ref DOSING_CYCLES_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("dosing_cycles_total", "Tổng số chu kỳ châm phân theo trigger và trạng thái"),
        &["trigger", "outcome"]
    ).expect("metric can be created");
}

pub fn register_metrics() {
    // Tự động thu thập metrics tiến trình OS (CPU, Memory, File Descriptors)
    #[cfg(target_os = "linux")]
    {
        let process_collector = prometheus::process_collector::ProcessCollector::for_self();
        let _ = REGISTRY.register(Box::new(process_collector));
    }

    REGISTRY
        .register(Box::new(HTTP_REQUESTS_TOTAL.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(HTTP_REQ_DURATION_SECONDS.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(ACTIVE_WS_CONNECTIONS.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(MQTT_MESSAGES_RECEIVED_TOTAL.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(MQTT_PROCESSING_ERRORS_TOTAL.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(SENSOR_UPDATES_TOTAL.clone()))
        .unwrap();
    REGISTRY
        .register(Box::new(DOSING_CYCLES_TOTAL.clone()))
        .unwrap();
}

pub fn gather_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap_or_default()
}
