// src/metrics.rs

use lazy_static::lazy_static;
use prometheus::{
    CounterVec, Encoder, GaugeVec, HistogramOpts, HistogramVec, IntCounter, IntCounterVec,
    IntGauge, IntGaugeVec, Opts, Registry, TextEncoder,
};

lazy_static! {
    pub static ref REGISTRY: Registry = Registry::new();

    // =========================================================================
    // 1. HTTP, WEBSOCKET & MQTT INFRASTRUCTURE METRICS
    // =========================================================================
    pub static ref HTTP_REQUESTS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("http_requests_total", "Tổng số HTTP requests nhận được"),
        &["method", "endpoint", "status"]
    ).expect("metric can be created");

    pub static ref HTTP_REQ_DURATION_SECONDS: HistogramVec = HistogramVec::new(
        HistogramOpts::new("http_request_duration_seconds", "Thời gian xử lý HTTP request (giây)")
            .buckets(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]),
        &["method", "endpoint"]
    ).expect("metric can be created");

    pub static ref ACTIVE_WS_CONNECTIONS: IntGauge = IntGauge::new(
        "active_ws_connections",
        "Số lượng kết nối WebSocket đang hoạt động"
    ).expect("metric can be created");

    pub static ref MQTT_MESSAGES_RECEIVED_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("mqtt_messages_received_total", "Tổng số message MQTT nhận được theo topic"),
        &["topic_suffix"]
    ).expect("metric can be created");

    pub static ref MQTT_PROCESSING_ERRORS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("mqtt_processing_errors_total", "Tổng số lỗi khi xử lý MQTT message"),
        &["topic_suffix", "error_type"]
    ).expect("metric can be created");

    pub static ref SENSOR_UPDATES_TOTAL: IntCounter = IntCounter::new(
        "sensor_updates_total",
        "Tổng số bản ghi dữ liệu cảm biến đã nhận từ Sensor Node"
    ).expect("metric can be created");

    pub static ref DOSING_CYCLES_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("dosing_cycles_total", "Tổng số chu kỳ châm phân theo trigger và trạng thái kết thúc"),
        &["trigger", "outcome"]
    ).expect("metric can be created");

    // =========================================================================
    // 2. ADAPTIVE LEARNING & DYNAMIC GAIN / STEP RATIO
    // =========================================================================
    pub static ref ADAPTIVE_GAIN_PER_ML: GaugeVec = GaugeVec::new(
        Opts::new("agitech_adaptive_gain_per_ml", "Hệ số đáp ứng hiệu quả thu được qua tự học EMA (gain/ml)"),
        &["device_id", "channel"] // channel: "ec", "ph_up", "ph_down"
    ).expect("metric can be created");

    pub static ref ADAPTIVE_STEP_RATIO: GaugeVec = GaugeVec::new(
        Opts::new("agitech_adaptive_step_ratio", "Hệ số bước châm thích ứng từ AutoTuner"),
        &["device_id", "parameter"] // parameter: "ec", "ph", "best_ec", "best_ph"
    ).expect("metric can be created");

    pub static ref ADAPTIVE_TUNER_STATE: IntGaugeVec = IntGaugeVec::new(
        Opts::new("agitech_adaptive_tuner_state", "Trạng thái AutoTuner (0: Exploring, 1: Converging, 2: Stable, 3: Degraded)"),
        &["device_id"]
    ).expect("metric can be created");

    pub static ref ADAPTIVE_EFFECTIVE_TOLERANCE: GaugeVec = GaugeVec::new(
        Opts::new("agitech_adaptive_effective_tolerance", "Ngưỡng sai số thích ứng động (Dynamic Tolerance)"),
        &["device_id", "parameter"] // parameter: "ec", "ph"
    ).expect("metric can be created");

    // =========================================================================
    // 3. MIMO INTERACTION MATRIX & KALMAN FILTER
    // =========================================================================
    pub static ref ADAPTIVE_MATRIX_IS_WARM: IntGaugeVec = IntGaugeVec::new(
        Opts::new("agitech_adaptive_matrix_is_warm", "Trạng thái ma trận MIMO (1: Warm/Đã hội tụ, 0: Cold/Khởi tạo)"),
        &["device_id"]
    ).expect("metric can be created");

    pub static ref ADAPTIVE_MATRIX_UPDATE_COUNT: IntGaugeVec = IntGaugeVec::new(
        Opts::new("agitech_adaptive_matrix_update_count", "Số chu kỳ học đã cập nhật vào ma trận tương tác MIMO"),
        &["device_id"]
    ).expect("metric can be created");

    pub static ref KALMAN_ACTUATOR_CONFIDENCE: GaugeVec = GaugeVec::new(
        Opts::new("agitech_kalman_actuator_confidence", "Độ tin cậy của bộ lọc Kalman đối với từng cơ cấu chấp hành (0.0 -> 1.0)"),
        &["device_id", "actuator"] // actuator: nutrient_a, nutrient_b, ph_up, ph_down, water_in, water_out, osaka_mixing, misting
    ).expect("metric can be created");

    // =========================================================================
    // 4. FLUID DYNAMICS & TIMING METRICS
    // =========================================================================
    pub static ref ADAPTIVE_FLUID_TIME_SECONDS: IntGaugeVec = IntGaugeVec::new(
        Opts::new("agitech_adaptive_fluid_time_seconds", "Thời gian thích ứng động của thủy lực (giây)"),
        &["device_id", "phase"] // phase: "mixing", "stabilizing"
    ).expect("metric can be created");

    pub static ref DOSING_CYCLE_PHASE_DURATION_SECONDS: HistogramVec = HistogramVec::new(
        HistogramOpts::new("agitech_dosing_cycle_phase_duration_seconds", "Phân phối thời gian thực tế của các giai đoạn chu kỳ châm (giây)")
            .buckets(vec![5.0, 10.0, 15.0, 20.0, 30.0, 45.0, 60.0, 90.0, 120.0, 180.0]),
        &["device_id", "phase"] // phase: "total", "mixing", "stabilizing"
    ).expect("metric can be created");

    // =========================================================================
    // 5. DOSING SNAPSHOTS (BEFORE / POST-MIX / POST-STABLE / DELTA / ERROR)
    // =========================================================================
    pub static ref DOSING_SNAPSHOT_EC: GaugeVec = GaugeVec::new(
        Opts::new("agitech_dosing_snapshot_ec", "Giá trị EC tại các mốc trong chu kỳ châm (mS/cm)"),
        &["device_id", "phase"] // phase: "pre", "post_mixing", "post_stable", "target", "delta", "error"
    ).expect("metric can be created");

    pub static ref DOSING_SNAPSHOT_PH: GaugeVec = GaugeVec::new(
        Opts::new("agitech_dosing_snapshot_ph", "Giá trị pH tại các mốc trong chu kỳ châm"),
        &["device_id", "phase"] // phase: "pre", "post_mixing", "post_stable", "target", "delta", "error"
    ).expect("metric can be created");

    pub static ref DOSING_SNAPSHOT_WATER_LEVEL: GaugeVec = GaugeVec::new(
        Opts::new("agitech_dosing_snapshot_water_level", "Mực nước tại các mốc trong chu kỳ châm (cm/%)"),
        &["device_id", "phase"] // phase: "pre", "post_mixing", "post_stable"
    ).expect("metric can be created");

    // =========================================================================
    // 6. DELIVERED VOLUMES & TOTAL CONSUMPTION
    // =========================================================================
    pub static ref DOSING_DELIVERED_DOSE_ML: GaugeVec = GaugeVec::new(
        Opts::new("agitech_dosing_delivered_dose_ml", "Thể tích dung dịch đã bơm trong chu kỳ gần nhất (ml)"),
        &["device_id", "pump"] // pump: "pump_a", "pump_b", "ph_up", "ph_down", "total_nutrient", "total_ph"
    ).expect("metric can be created");

    pub static ref DOSING_WATER_ACTUATOR_SECONDS: GaugeVec = GaugeVec::new(
        Opts::new("agitech_dosing_water_actuator_seconds", "Thời gian chạy bơm cấp/thoát nước trong chu kỳ (giây)"),
        &["device_id", "direction"] // direction: "water_in", "water_out"
    ).expect("metric can be created");

    pub static ref DOSING_PUMP_TOTAL_ML: CounterVec = CounterVec::new(
        Opts::new("agitech_dosing_pump_total_ml", "Tổng thể tích dung dịch đã tiêu thụ lũy kế (ml)"),
        &["device_id", "pump"] // pump: "pump_a", "pump_b", "ph_up", "ph_down"
    ).expect("metric can be created");

    // =========================================================================
    // 7. SAFETY BUDGETS & HOURLY LIMITS
    // =========================================================================
    pub static ref SAFETY_HOURLY_DOSE_ML: GaugeVec = GaugeVec::new(
        Opts::new("agitech_safety_hourly_dose_ml", "Tổng lượng phân/axit đã châm trong cửa sổ trượt 1 giờ qua (ml)"),
        &["device_id", "type"] // type: "ec", "ph"
    ).expect("metric can be created");

    pub static ref SAFETY_HOURLY_WATER_CYCLES: IntGaugeVec = IntGaugeVec::new(
        Opts::new("agitech_safety_hourly_water_cycles", "Số lần cấp/xả nước trong cửa sổ trượt 1 giờ qua"),
        &["device_id", "type"] // type: "refill", "drain"
    ).expect("metric can be created");

    // =========================================================================
    // 8. FAULT DIAGNOSTICS & RESIDUAL STREAKS
    // =========================================================================
    pub static ref DIAGNOSTIC_FAULT_STREAK: IntGaugeVec = IntGaugeVec::new(
        Opts::new("agitech_diagnostic_fault_streak", "Số lần liên tiếp bơm chạy nhưng cảm biến không phản hồi"),
        &["device_id", "subsystem"] // subsystem: "ec_pump", "ph_pump", "water_hydraulics"
    ).expect("metric can be created");

    // =========================================================================
    // 9. CONTROLLER NODE HARDWARE & SYSTEM TELEMETRY
    // =========================================================================
    pub static ref CONTROLLER_FREE_HEAP_BYTES: IntGaugeVec = IntGaugeVec::new(
        Opts::new("agitech_controller_free_heap_bytes", "Bộ nhớ RAM Heap còn trống của ESP32 (bytes)"),
        &["device_id"]
    ).expect("metric can be created");

    pub static ref CONTROLLER_WIFI_RSSI_DBM: IntGaugeVec = IntGaugeVec::new(
        Opts::new("agitech_controller_wifi_rssi_dbm", "Cường độ tín hiệu sóng WiFi của Controller Node (dBm)"),
        &["device_id"]
    ).expect("metric can be created");

    pub static ref CONTROLLER_UPTIME_SECONDS: IntGaugeVec = IntGaugeVec::new(
        Opts::new("agitech_controller_uptime_seconds", "Thời gian hoạt động liên tục của Controller Node (giây)"),
        &["device_id"]
    ).expect("metric can be created");

    pub static ref CONTROLLER_LOG_DROPPED_TOTAL: IntGaugeVec = IntGaugeVec::new(
        Opts::new("agitech_controller_log_dropped_total", "Số log bị huỷ do đầy hàng đợi trên Controller Node"),
        &["device_id"]
    ).expect("metric can be created");

    // =========================================================================
    // 10. HESTIA BIO-COMFORT ENGINE METRICS
    // =========================================================================
    pub static ref HESTIA_HEALTH_SCORE: IntGaugeVec = IntGaugeVec::new(
        Opts::new("agitech_hestia_health_score_percent", "Điểm đánh giá sức khỏe sinh học cây trồng của Hestia Engine (0-100)"),
        &["device_id"]
    ).expect("metric can be created");

    pub static ref HESTIA_CONFIDENCE: GaugeVec = GaugeVec::new(
        Opts::new("agitech_hestia_confidence", "Độ tin cậy của mô hình đánh giá sinh học Hestia (0.0 - 1.0)"),
        &["device_id"]
    ).expect("metric can be created");

    pub static ref HESTIA_AXIS_COMFORT: GaugeVec = GaugeVec::new(
        Opts::new("agitech_hestia_axis_comfort", "Điểm mức độ tối ưu từng trục môi trường (0.0 - 1.0)"),
        &["device_id", "axis"] // axis: "ec", "ph", "water_level", "temp"
    ).expect("metric can be created");

    pub static ref HESTIA_AXIS_WEIGHT: GaugeVec = GaugeVec::new(
        Opts::new("agitech_hestia_axis_weight", "Trọng số động của từng trục môi trường theo Hestia Engine"),
        &["device_id", "axis"] // axis: "ec", "ph", "water_level", "temp"
    ).expect("metric can be created");

    pub static ref HESTIA_AXIS_ACTION_FACTOR: GaugeVec = GaugeVec::new(
        Opts::new("agitech_hestia_axis_action_factor", "Hệ số can thiệp phục hồi của trục môi trường (Action Factor)"),
        &["device_id", "axis"] // axis: "ec", "ph", "water_level", "temp"
    ).expect("metric can be created");
}

static INIT: std::sync::Once = std::sync::Once::new();

pub fn register_metrics() {
    INIT.call_once(|| {
        // Thu thập metrics tiến trình OS (CPU, Memory, File Descriptors) trên Linux
        #[cfg(target_os = "linux")]
        {
            let process_collector = prometheus::process_collector::ProcessCollector::for_self();
            let _ = REGISTRY.register(Box::new(process_collector));
        }

        // 1. Basic Infrastructure
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

        // 2. Adaptive Learning
        REGISTRY
            .register(Box::new(ADAPTIVE_GAIN_PER_ML.clone()))
            .unwrap();
        REGISTRY
            .register(Box::new(ADAPTIVE_STEP_RATIO.clone()))
            .unwrap();
        REGISTRY
            .register(Box::new(ADAPTIVE_TUNER_STATE.clone()))
            .unwrap();
        REGISTRY
            .register(Box::new(ADAPTIVE_EFFECTIVE_TOLERANCE.clone()))
            .unwrap();

        // 3. Matrix & Kalman
        REGISTRY
            .register(Box::new(ADAPTIVE_MATRIX_IS_WARM.clone()))
            .unwrap();
        REGISTRY
            .register(Box::new(ADAPTIVE_MATRIX_UPDATE_COUNT.clone()))
            .unwrap();
        REGISTRY
            .register(Box::new(KALMAN_ACTUATOR_CONFIDENCE.clone()))
            .unwrap();

        // 4. Fluid & Timing
        REGISTRY
            .register(Box::new(ADAPTIVE_FLUID_TIME_SECONDS.clone()))
            .unwrap();
        REGISTRY
            .register(Box::new(DOSING_CYCLE_PHASE_DURATION_SECONDS.clone()))
            .unwrap();

        // 5. Dosing Snapshots
        REGISTRY
            .register(Box::new(DOSING_SNAPSHOT_EC.clone()))
            .unwrap();
        REGISTRY
            .register(Box::new(DOSING_SNAPSHOT_PH.clone()))
            .unwrap();
        REGISTRY
            .register(Box::new(DOSING_SNAPSHOT_WATER_LEVEL.clone()))
            .unwrap();

        // 6. Delivered Volumes & Total
        REGISTRY
            .register(Box::new(DOSING_DELIVERED_DOSE_ML.clone()))
            .unwrap();
        REGISTRY
            .register(Box::new(DOSING_WATER_ACTUATOR_SECONDS.clone()))
            .unwrap();
        REGISTRY
            .register(Box::new(DOSING_PUMP_TOTAL_ML.clone()))
            .unwrap();

        // 7. Safety Budgets
        REGISTRY
            .register(Box::new(SAFETY_HOURLY_DOSE_ML.clone()))
            .unwrap();
        REGISTRY
            .register(Box::new(SAFETY_HOURLY_WATER_CYCLES.clone()))
            .unwrap();

        // 8. Fault Diagnostics
        REGISTRY
            .register(Box::new(DIAGNOSTIC_FAULT_STREAK.clone()))
            .unwrap();

        // 9. Hardware & Node Telemetry
        REGISTRY
            .register(Box::new(CONTROLLER_FREE_HEAP_BYTES.clone()))
            .unwrap();
        REGISTRY
            .register(Box::new(CONTROLLER_WIFI_RSSI_DBM.clone()))
            .unwrap();
        REGISTRY
            .register(Box::new(CONTROLLER_UPTIME_SECONDS.clone()))
            .unwrap();
        REGISTRY
            .register(Box::new(CONTROLLER_LOG_DROPPED_TOTAL.clone()))
            .unwrap();

        // 10. Hestia Bio-Comfort Assessment
        REGISTRY
            .register(Box::new(HESTIA_HEALTH_SCORE.clone()))
            .unwrap();
        REGISTRY
            .register(Box::new(HESTIA_CONFIDENCE.clone()))
            .unwrap();
        REGISTRY
            .register(Box::new(HESTIA_AXIS_COMFORT.clone()))
            .unwrap();
        REGISTRY
            .register(Box::new(HESTIA_AXIS_WEIGHT.clone()))
            .unwrap();
        REGISTRY
            .register(Box::new(HESTIA_AXIS_ACTION_FACTOR.clone()))
            .unwrap();
    });
}

pub fn gather_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = REGISTRY.gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_register_metrics() {
        // Register metrics multiple times to ensure no panics occur due to duplicate registration
        register_metrics();
        register_metrics();

        let families = REGISTRY.gather();
        let metric_names: Vec<String> = families.iter().map(|f| f.get_name().to_string()).collect();

        // Assert that at least some of our custom metrics are registered
        // (If another test runs first and fails registration because unwrap was used,
        //  it would panic there. Since we use ok(), we just want to make sure
        //  our custom metrics made it in at some point).

        // Note: active_ws_connections is one of our custom metrics
        assert!(
            metric_names.contains(&"active_ws_connections".to_string()),
            "Custom metric active_ws_connections should be registered. Found: {:?}",
            metric_names
        );

        // And check for sensor_updates_total
        assert!(
            metric_names.contains(&"sensor_updates_total".to_string()),
            "Custom metric sensor_updates_total should be registered. Found: {:?}",
            metric_names
        );
    }
}
