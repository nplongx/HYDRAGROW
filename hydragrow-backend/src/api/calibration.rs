use std::cmp::Ordering;
use std::collections::HashSet;
use std::time::Duration;

use actix_web::{HttpResponse, Responder, web};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{AppState, PhCalibrationMode, PhCalibrationSession, PhCapturedPoint, PhVoltageSample};

const DEFAULT_SAMPLE_TARGET: usize = 20;
const DEFAULT_WINDOW_SECONDS: i64 = 10;
const SESSION_TIMEOUT_SECONDS: i64 = 300;
const MAX_BUFFER_AGE_SECONDS: i64 = 120;

#[derive(Debug, Deserialize)]
pub struct StartPhCalibrationRequest {
    pub mode: String,
    pub timeout_seconds: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CapturePhPointRequest {
    pub point: i32,
    pub sample_target: Option<usize>,
    pub window_seconds: Option<i64>,
}

#[derive(Debug, Serialize)]
struct CapturePhPointResponse {
    point: i32,
    mean_voltage_mv: f64,
    sample_count: usize,
    captured_at: chrono::DateTime<chrono::Utc>,
    window_seconds: i64,
}

#[derive(Debug, Serialize)]
struct FinishPhCalibrationResponse {
    mode: &'static str,
    ph_v7: f64,
    ph_v4: f64,
    ph_v10: Option<f64>,
    residual: Option<f64>,
    error: Option<String>,
    timestamp: chrono::DateTime<chrono::Utc>,
    captured_points: Vec<PhCapturedPoint>,
}

fn parse_mode(mode: &str) -> Option<PhCalibrationMode> {
    match mode {
        "2-point" | "2_point" | "2point" | "two-point" => Some(PhCalibrationMode::TwoPoint),
        "3-point" | "3_point" | "3point" | "three-point" => Some(PhCalibrationMode::ThreePoint),
        _ => None,
    }
}

fn required_points(mode: &PhCalibrationMode) -> HashSet<i32> {
    match mode {
        PhCalibrationMode::TwoPoint => HashSet::from([4, 7]),
        PhCalibrationMode::ThreePoint => HashSet::from([4, 7, 10]),
    }
}

fn trim_samples(
    samples: &mut std::collections::VecDeque<PhVoltageSample>,
    now: chrono::DateTime<Utc>,
) {
    samples.retain(|sample| (now - sample.observed_at).num_seconds() <= MAX_BUFFER_AGE_SECONDS);
}

fn reject_outliers(mut values: Vec<f64>) -> Vec<f64> {
    if values.len() < 4 {
        return values;
    }

    values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    let q1_idx = values.len() / 4;
    let q3_idx = (values.len() * 3) / 4;

    let q1 = values[q1_idx];
    let q3 = values[q3_idx];
    let iqr = q3 - q1;
    if iqr <= f64::EPSILON {
        return values;
    }

    let lower = q1 - 1.5 * iqr;
    let upper = q3 + 1.5 * iqr;
    values
        .into_iter()
        .filter(|v| *v >= lower && *v <= upper)
        .collect()
}

pub async fn start_ph_calibration(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
    req: web::Json<StartPhCalibrationRequest>,
) -> impl Responder {
    let device_id = path.into_inner();
    let mode = match parse_mode(req.mode.trim()) {
        Some(mode) => mode,
        None => {
            return HttpResponse::BadRequest().json(json!({
                "error": "invalid_mode",
                "message": "mode phải là 2-point hoặc 3-point"
            }));
        }
    };

    let timeout = req
        .timeout_seconds
        .unwrap_or(SESSION_TIMEOUT_SECONDS)
        .clamp(30, 1800);
    let now = Utc::now();

    let mut sessions = app_state.ph_calibration_sessions.write().await;
    sessions.insert(
        device_id.clone(),
        PhCalibrationSession {
            mode: mode.clone(),
            started_at: now,
            expires_at: now + chrono::Duration::seconds(timeout),
            captured_points: Default::default(),
        },
    );

    let mode_text = if mode == PhCalibrationMode::TwoPoint {
        "2-point"
    } else {
        "3-point"
    };

    HttpResponse::Ok().json(json!({
        "status": "success",
        "device_id": device_id,
        "mode": mode_text,
        "started_at": now,
        "expires_at": now + chrono::Duration::seconds(timeout)
    }))
}

pub async fn capture_ph_calibration_point(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
    req: web::Json<CapturePhPointRequest>,
) -> impl Responder {
    let device_id = path.into_inner();
    let point = req.point;
    if ![4, 7, 10].contains(&point) {
        return HttpResponse::BadRequest().json(json!({
            "error": "invalid_point",
            "message": "point phải là 4, 7 hoặc 10"
        }));
    }

    let now = Utc::now();

    {
        let mut sessions = app_state.ph_calibration_sessions.write().await;
        match sessions.get_mut(&device_id) {
            Some(session) => {
                if session.expires_at < now {
                    sessions.remove(&device_id);
                    return HttpResponse::BadRequest().json(json!({
                        "error": "session_expired",
                        "message": "Phiên calibration đã hết hạn, vui lòng start lại"
                    }));
                }
                if !required_points(&session.mode).contains(&point) {
                    return HttpResponse::BadRequest().json(json!({
                        "error": "point_not_allowed",
                        "message": "Point không hợp lệ với mode hiện tại"
                    }));
                }
                let _ = &session.mode;
            }
            None => {
                return HttpResponse::BadRequest().json(json!({
                    "error": "missing_session",
                    "message": "Chưa start calibration cho device này"
                }));
            }
        }
    };

    let sample_target = req
        .sample_target
        .unwrap_or(DEFAULT_SAMPLE_TARGET)
        .clamp(5, 200);
    let window_seconds = req
        .window_seconds
        .unwrap_or(DEFAULT_WINDOW_SECONDS)
        .clamp(3, 60);
    let window_duration = chrono::Duration::seconds(window_seconds);

    tokio::time::sleep(Duration::from_secs(window_seconds as u64)).await;

    let sampling_now = Utc::now();
    let filtered_values = {
        let mut all_samples = app_state.ph_voltage_samples.write().await;
        let samples = match all_samples.get_mut(&device_id) {
            Some(v) => v,
            None => {
                return HttpResponse::BadRequest().json(json!({
                    "error": "missing_sensor_stream",
                    "message": "Chưa có dữ liệu ph_voltage_mv từ stream sensor"
                }));
            }
        };
        trim_samples(samples, sampling_now);
        let mut values: Vec<f64> = samples
            .iter()
            .filter(|s| sampling_now - s.observed_at <= window_duration)
            .map(|s| s.voltage_mv)
            .collect();
        values.sort_by(|a, b| b.partial_cmp(a).unwrap_or(Ordering::Equal));
        if values.len() > sample_target {
            values.truncate(sample_target);
        }
        reject_outliers(values)
    };

    if filtered_values.len() < sample_target {
        return HttpResponse::BadRequest().json(json!({
            "error": "insufficient_samples",
            "message": "Không đủ mẫu ph_voltage_mv trong khoảng thời gian yêu cầu",
            "required": sample_target,
            "actual": filtered_values.len(),
            "window_seconds": window_seconds
        }));
    }

    let mean_voltage_mv = filtered_values.iter().sum::<f64>() / filtered_values.len() as f64;
    let captured_at = Utc::now();

    let mut sessions = app_state.ph_calibration_sessions.write().await;
    if let Some(session) = sessions.get_mut(&device_id) {
        session.captured_points.insert(
            point,
            PhCapturedPoint {
                point,
                voltage_mv: mean_voltage_mv,
                sample_count: filtered_values.len(),
                captured_at,
            },
        );
    }

    HttpResponse::Ok().json(json!({
        "status": "success",
        "data": CapturePhPointResponse {
            point,
            mean_voltage_mv,
            sample_count: filtered_values.len(),
            captured_at,
            window_seconds,
        }
    }))
}

pub async fn finish_ph_calibration(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> impl Responder {
    let device_id = path.into_inner();
    let now = Utc::now();

    let session = {
        let mut sessions = app_state.ph_calibration_sessions.write().await;
        match sessions.remove(&device_id) {
            Some(session) => session,
            None => {
                return HttpResponse::BadRequest().json(json!({
                    "error": "missing_session",
                    "message": "Chưa có session calibration để finish"
                }));
            }
        }
    };

    if session.expires_at < now {
        return HttpResponse::BadRequest().json(json!({
            "error": "session_expired",
            "message": "Phiên calibration đã hết hạn"
        }));
    }

    let required = required_points(&session.mode);
    for point in &required {
        if !session.captured_points.contains_key(point) {
            return HttpResponse::BadRequest().json(json!({
                "error": "missing_point",
                "message": format!("Thiếu điểm pH {}. Vui lòng capture đủ trước khi finish", point),
            }));
        }
    }

    let p7 = session
        .captured_points
        .get(&7)
        .map(|v| v.voltage_mv)
        .unwrap_or(0.0);
    let p4 = session
        .captured_points
        .get(&4)
        .map(|v| v.voltage_mv)
        .unwrap_or(0.0);
    let p10 = session.captured_points.get(&10).map(|v| v.voltage_mv);

    let mut residual = None;
    let mut error = None;

    if let Some(v10) = p10 {
        let slope = (p4 - p7) / (4.0 - 7.0);
        let predicted_v10 = p7 + slope * (10.0 - 7.0);
        let r = v10 - predicted_v10;
        residual = Some(r);
        error = Some(format!("abs_error_mv={:.3}", r.abs()));
    }

    let mode_text = if session.mode == PhCalibrationMode::TwoPoint {
        "2-point"
    } else {
        "3-point"
    };

    let mut captured_points: Vec<PhCapturedPoint> = session.captured_points.into_values().collect();
    captured_points.sort_by_key(|p| p.point);

    HttpResponse::Ok().json(json!({
        "status": "success",
        "data": FinishPhCalibrationResponse {
            mode: mode_text,
            ph_v7: p7,
            ph_v4: p4,
            ph_v10: p10,
            residual,
            error,
            timestamp: now,
            captured_points,
        }
    }))
}

pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.route(
        "/calibration/ph/start",
        web::post().to(start_ph_calibration),
    )
    .route(
        "/calibration/ph/capture",
        web::post().to(capture_ph_calibration_point),
    )
    .route(
        "/calibration/ph/finish",
        web::post().to(finish_ph_calibration),
    );
}
