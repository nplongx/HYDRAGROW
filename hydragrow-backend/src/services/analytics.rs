//! Tổng hợp số liệu cho nhóm block Adaptive/Analytics (nhóm ⑦ đề xuất gốc). Thuần
//! Rust — không I/O — để test được mà không cần InfluxDB/Postgres thật.

use hydragrow_shared::telemetry::cycle::KalmanLearningData;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct RangeStats {
    pub mean: f64,
    pub min: f64,
    pub max: f64,
    pub count: usize,
}

/// Trả `None` nếu không có điểm dữ liệu nào trong khoảng — caller (API handler)
/// tự quyết định trả 404 hay một giá trị mặc định, hàm này không đoán thay.
pub fn compute_range_stats(values: &[f64]) -> Option<RangeStats> {
    if values.is_empty() {
        return None;
    }
    let count = values.len();
    let sum: f64 = values.iter().sum();
    let mean = sum / count as f64;
    let min = values.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = values.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    Some(RangeStats {
        mean,
        min,
        max,
        count,
    })
}

/// `dosing_reports.payload` là nguyên văn `DosingCycleEvent` JSON (xem
/// `mqtt/handlers/dosing_cycle.rs::insert_dosing_report`). Trả `None` nếu thiếu
/// key `kalman` (chu kỳ không bật adaptive learning) HOẶC nếu key có mặt nhưng
/// không parse được đúng shape — không panic trên dữ liệu lịch sử cũ/lỗi định dạng.
pub fn extract_kalman_from_payload(payload: &serde_json::Value) -> Option<KalmanLearningData> {
    payload
        .get("kalman")
        .and_then(|v| serde_json::from_value::<KalmanLearningData>(v.clone()).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_kalman_data_when_present_in_payload() {
        let payload = serde_json::json!({
            "cycle_id": "c1",
            "kalman": {
                "ec_gain_before": 0.01, "ec_gain_after": 0.012,
                "ph_up_gain_before": 0.02, "ph_up_gain_after": 0.021,
                "ph_down_gain_before": 0.02, "ph_down_gain_after": 0.019,
                "matrix_update_count": 5, "matrix_is_warm": true,
                "adaptive_mixing_sec": 30, "adaptive_stabilize_sec": 20
            }
        });
        let kalman = extract_kalman_from_payload(&payload).unwrap();
        assert_eq!(kalman.matrix_update_count, 5);
        assert!(kalman.matrix_is_warm);
    }

    #[test]
    fn returns_none_when_kalman_field_absent() {
        let payload = serde_json::json!({ "cycle_id": "c1" });
        assert!(extract_kalman_from_payload(&payload).is_none());
    }

    #[test]
    fn returns_none_for_malformed_kalman_field_instead_of_panicking() {
        let payload = serde_json::json!({ "cycle_id": "c1", "kalman": "not-an-object" });
        assert!(extract_kalman_from_payload(&payload).is_none());
    }

    #[test]
    fn computes_mean_min_max_count_for_nonempty_values() {
        let stats = compute_range_stats(&[1.0, 2.0, 3.0, 4.0]).unwrap();
        assert_eq!(stats.count, 4);
        assert!((stats.mean - 2.5).abs() < 1e-9);
        assert_eq!(stats.min, 1.0);
        assert_eq!(stats.max, 4.0);
    }

    #[test]
    fn returns_none_for_empty_values() {
        assert!(compute_range_stats(&[]).is_none());
    }

    #[test]
    fn handles_a_single_value() {
        let stats = compute_range_stats(&[6.5]).unwrap();
        assert_eq!(stats.count, 1);
        assert_eq!(stats.mean, 6.5);
        assert_eq!(stats.min, 6.5);
        assert_eq!(stats.max, 6.5);
    }
}
