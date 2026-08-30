Phase 1 — Action Blocks (DosingActor / WaterActor / Emergency Stop) Implementation Plan

For agentic workers: Chạy sau khi 2 plan Phase 0 đã merge vào main (2026-08-29-automation-foundation-safety-mirror.md và 2026-08-29-flow-blockly-drilldown-ux.md). Task 1–4 (backend) và Task 5–8 (frontend) có thể chạy song song ở 2 lane khác nhau — xem mục “Parallel lanes” ở cuối file.

Correction trước khi bắt đầu (đọc kỹ, đổi hành vi so với Phase 0)
Khi đào sâu vào api/control.rs (endpoint điều khiển thủ công đã chạy production) để lấy field names chính xác cho Action blocks, phát hiện:
ActionCommandOutput → CommandPayload mà Plan A Task 6 viết là sai định dạng dây (wire format).
services::command::CommandPayload { action, pump, duration_sec } — không có field pwm, không có field nào chứa dose_ml. Task 6 của Plan A gọi to_command_payload(&output) và âm thầm làm rơi mất dose_ml — bug thật, không phải giả định sai vô hại.
Dosing trên HYDRAGROW là PWM + thời lượng, không phải thể tích trực tiếp: estimated_ml = capacity_ml_per_sec * (pwm/100) * duration_sec — công thức này đã tồn tại và đang chạy thật trong validate_manual_dose_safety (api/control.rs). Muốn bơm đúng dose_ml, phải tính ngược ra duration_sec theo calibration của đúng pump, KHÔNG được đoán.
Wire format đang thực sự chạy cho lệnh bơm/van là hydragrow_shared::{MqttCommandOut, MqttCommandParams} (có pwm, state, ota_url…), publish qua api::mqtt_utils::publish_command — không phải CommandPayload (dùng services::command::send_command). Hai đường publish cùng ký tên bằng sign_command nhưng khác shape payload. CommandPayload hiện chỉ còn được xác nhận dùng cho đúng 1 việc: trigger_emergency_stop.

Quyết định (rủi ro thấp nhất, không đoán hành vi firmware chưa xác nhận):
- action_command output "emergency_stop" → vẫn gọi trigger_emergency_stop() y nguyên (đường đã chạy, không đổi).
- "dose" / "water_on" / "water_off" → build MqttCommandOut với mqtt_action đúng 3 giá trị đã xác nhận có thật trong api/control.rs ("set_pwm", "pump_on", "pump_off"), publish qua publish_command — tái dùng chính route đã chạy cho manual control, không phát minh action string mới.
- “Chuyển FSM state (skip mixing…)” và “Trigger OTA update firmware” từ đề xuất gốc: không có action string nào được xác nhận trong repo cho 2 việc này (không tìm thấy handler nào ngoài emergency_stop/reset_fault/force_on/pump_on/pump_off/set_pwm). Để ngoài scope Phase 1, dời sang khi có xác nhận từ người phụ trách ESP32-C3-CONTROLLER-NODE — đúng tinh thần thận trọng đã áp dụng cho “gain learner reset” ở roadmap.
- Roadmap trước đó ghi “Phase 1 dùng registry pattern từ Plan B” — đính chính: Plan B khi viết thực tế không làm refactor registry (chỉ làm hydrate + drilldown UX). Phase 1 chỉ thêm 1 nhóm block (Action) trong 1 lane duy nhất nên chưa có xung đột merge thật — registry refactor được dời tới trước khi Phase 2 chạy song song với Phase khác (ghi lại trong roadmap ở Task 8).

Goal: Hoàn thiện pipeline Action blocks từ Blockly → Rhai → ActionCommandOutput (đã có PWM) → safety gate → MqttCommandOut thật, cho 3 hành động: Dose (DosingActor), Water on/off (WaterActor), Emergency stop.
Tech Stack: Rust (sqlx, rhai), TypeScript (zod, blockly).

Task 1: Module hydragrow-shared::dosing — công thức ml ↔ duration thuần
Files:
- Create: hydragrow-shared/src/dosing.rs
- Modify: hydragrow-shared/src/lib.rs

Step 1: Viết failing test
// hydragrow-shared/src/dosing.rs
#[cfg(test)]
mod tests {
use super::*;
#[test]
fn estimate_ml_matches_manual_control_formula() {
// capacity=1.2ml/s, pwm=50%, 10s → 1.2 * 0.5 * 10 = 6.0ml
// (đúng công thức trong api/control.rs::validate_manual_dose_safety)
assert!((estimate_ml(1.2, 50, 10) - 6.0).abs() < 1e-4);
}
#[test]
fn ml_to_duration_sec_is_inverse_of_estimate_ml() {
let duration = ml_to_duration_sec(1.2, 50, 6.0).unwrap();
assert_eq!(duration, 10);
}
#[test]
fn ml_to_duration_sec_rounds_up_so_dose_is_never_under_delivered() {
// 1.2 * 1.0 * duration = 5.0 → duration = 4.1666...s → phải làm tròn LÊN thành 5s
let duration = ml_to_duration_sec(1.2, 100, 5.0).unwrap();
assert_eq!(duration, 5);
}
#[test]
fn ml_to_duration_sec_returns_none_for_zero_capacity() {
assert_eq!(ml_to_duration_sec(0.0, 100, 5.0), None);
}
#[test]
fn ml_to_duration_sec_returns_none_for_zero_pwm() {
assert_eq!(ml_to_duration_sec(1.2, 0, 5.0), None);
}
#[test]
fn normalize_dosing_pump_name_accepts_legacy_and_canonical_aliases() {
assert_eq!(normalize_dosing_pump_name("A"), Some("PUMP_A"));
assert_eq!(normalize_dosing_pump_name("PUMP_A"), Some("PUMP_A"));
assert_eq!(normalize_dosing_pump_name("PH_DOWN"), Some("PH_DOWN"));
assert_eq!(normalize_dosing_pump_name("NOT_A_PUMP"), None);
}
#[test]
fn capacity_ml_per_sec_for_pump_picks_the_right_field() {
assert_eq!(capacity_ml_per_sec_for_pump(1.0, 2.0, 3.0, 4.0, "PH_DOWN"), 4.0);
assert_eq!(capacity_ml_per_sec_for_pump(1.0, 2.0, 3.0, 4.0, "UNKNOWN"), 0.0);
}
}

Step 2: Chạy test để xác nhận fail
Run: cd hydragrow-shared && cargo test dosing:: -- --test-threads=1
Expected: FAIL biên dịch — module chưa tồn tại.

Step 3: Viết implementation
// hydragrow-shared/src/dosing.rs — thêm phía trên mod tests
//! Công thức thể tích ↔ thời lượng bơm định lượng, thuần Rust — mirror của công
//! thức đã chạy thật trong `hydragrow-backend/src/api/control.rs::
//! validate_manual_dose_safety` (`capacity_ml_per_sec * (pwm/100) * duration_sec`).
//! Đặt ở đây để Action blocks (script `action_command`) và endpoint manual-control
//! dùng CHUNG một công thức, không lệch nhau theo module-rules/shared.md rule #2.
//! `api/control.rs` hiện vẫn giữ bản copy riêng (không refactor trong Phase này để
//! tránh đụng vào code manual-control đã test kỹ) — ghi nhận là tech-debt nhỏ.
/// Ước lượng ml sẽ được bơm ra với PWM và thời lượng cho trước.
pub fn estimate_ml(capacity_ml_per_sec: f32, pwm_percent: u32, duration_sec: u64) -> f32 {
capacity_ml_per_sec * (pwm_percent as f32 / 100.0) * duration_sec as f32
}
/// Nghịch đảo của `estimate_ml`: cần bơm bao nhiêu giây để đạt `target_ml` ở PWM
/// cho trước. Làm tròn LÊN (ceil) — thà bơm dư một chút thời lượng còn hơn bơm
/// thiếu so với yêu cầu (an toàn hơn cho phía "không đủ liều" chứ không phải phía
/// ngược lại; `check_dose` ở `safety.rs` vẫn chặn nếu tổng vượt ngưỡng).
/// Trả `None` nếu capacity hoặc pwm bằng 0 (không thể bơm được gì).
pub fn ml_to_duration_sec(capacity_ml_per_sec: f32, pwm_percent: u32, target_ml: f32) -> Option<u64> {
if capacity_ml_per_sec <= 0.0 || pwm_percent == 0 {
return None;
}
let rate_ml_per_sec = capacity_ml_per_sec * (pwm_percent as f32 / 100.0);
if rate_ml_per_sec <= 0.0 {
return None;
}
Some((target_ml / rate_ml_per_sec).ceil() as u64)
}
/// Mirror của `normalize_dosing_pump_name` (private) trong `api/control.rs` —
/// đặt lại ở đây vì action_command dispatch (backend) cần cùng logic mà không
/// được phép import 1 hàm private từ module khác.
pub fn normalize_dosing_pump_name(pump: &str) -> Option<&'static str> {
match pump {
"A" | "PUMP_A" => Some("PUMP_A"),
"B" | "PUMP_B" => Some("PUMP_B"),
"PH_UP" => Some("PH_UP"),
"PH_DOWN" => Some("PH_DOWN"),
_ => None,
}
}
/// Nhận 4 field capacity rời (không nhận `DosingCalibration` struct — struct đó
/// sống ở `hydragrow-backend`, và `hydragrow-shared` không được phép phụ thuộc
/// ngược lại backend theo module-rules).
pub fn capacity_ml_per_sec_for_pump(
pump_a: f32,
pump_b: f32,
ph_up: f32,
ph_down: f32,
normalized_pump: &str,
) -> f32 {
match normalized_pump {
"PUMP_A" => pump_a,
"PUMP_B" => pump_b,
"PH_UP" => ph_up,
"PH_DOWN" => ph_down,
_ => 0.0,
}
}

// hydragrow-shared/src/lib.rs — thêm cạnh pub mod safety;
pub mod dosing;

Step 4: Chạy test để xác nhận pass
Run: cd hydragrow-shared && cargo test dosing:: -- --test-threads=1
Expected: PASS (7 tests)

Step 5: Commit
git add hydragrow-shared/src/dosing.rs hydragrow-shared/src/lib.rs
git commit -m "feat(shared): add pure ml<->duration dosing formula mirroring manual-control"

Task 2: db::postgres::fetch_dosing_calibration
Files:
- Modify: hydragrow-backend/src/db/postgres.rs
- Modify: hydragrow-backend/src/db/tests/test_postgres.rs

Step 1: Viết failing test
// hydragrow-backend/src/db/tests/test_postgres.rs — thêm vào cuối file
#[sqlx::test]
async fn fetch_dosing_calibration_returns_none_when_missing(pool: sqlx::PgPool) {
let result = fetch_dosing_calibration(&pool, "no-such-device").await.unwrap();
assert!(result.is_none());
}
#[sqlx::test]
async fn fetch_dosing_calibration_returns_row_when_present(pool: sqlx::PgPool) {
let cfg = DeviceConfig {
device_id: "dev-cal-1".to_string(),
ec_target: 1.8,
ec_tolerance: 0.2,
ph_target: 6.0,
ph_tolerance: 0.3,
control_mode: "auto".to_string(),
is_enabled: true,
delay_between_a_and_b_sec: 5,
last_updated: chrono::Utc::now(),
};
upsert_device_config(&pool, &cfg).await.unwrap(); // FK prerequisite
sqlx::query(
r#"
INSERT INTO dosing_calibration (
device_id, ec_gain_per_ml, ph_shift_up_per_ml, ph_shift_down_per_ml,
active_mixing_sec, sensor_stabilize_sec, ec_step_ratio, ph_step_ratio,
pump_a_capacity_ml_per_sec, pump_b_capacity_ml_per_sec,
pump_ph_up_capacity_ml_per_sec, pump_ph_down_capacity_ml_per_sec,
soft_start_duration, last_calibrated,
scheduled_mixing_interval_sec, scheduled_mixing_duration_sec,
dosing_pwm_percent, osaka_mixing_pwm_percent, osaka_misting_pwm_percent,
dosing_min_pwm_percent, pump_a_min_pwm_percent, pump_b_min_pwm_percent,
pump_ph_up_min_pwm_percent, pump_ph_down_min_pwm_percent, dosing_pulse_on_ms,
dosing_pulse_off_ms, dosing_min_dose_ml, dosing_max_pulse_count_per_cycle
) VALUES (
'dev-cal-1', 0.01, 0.01, 0.01,
300, 60, 1.0, 1.0,
1.2, 1.2,
0.8, 0.8,
3000, NOW(),
3600, 300,
50, 60, 100,
10, 10, 10,
10, 10, 200,
200, 0.1, 20
)
"#,
)
.execute(&pool)
.await
.unwrap();
let fetched = fetch_dosing_calibration(&pool, "dev-cal-1").await.unwrap().unwrap();
assert_eq!(fetched.device_id, "dev-cal-1");
assert!((fetched.pump_ph_down_capacity_ml_per_sec - 0.8).abs() < f32::EPSILON);
}

Step 2: Chạy test để xác nhận fail
Run: cd hydragrow-backend && cargo test fetch_dosing_calibration -- --test-threads=1
Expected: FAIL biên dịch — hàm chưa tồn tại.

Step 3: Viết implementation
// hydragrow-backend/src/db/postgres.rs — thêm cạnh get_safety_config
// Đặt tên `fetch_` (không phải `get_dosing_calibration`) để không đụng tên với
// handler HTTP `api::config::get_dosing_calibration` (impl Responder, nhận
// web::Path/web::Data — không gọi trực tiếp được từ code không phải actix
// extractor). Đây là bản callable-directly, tương tự cách `api/control.rs` từng
// tự viết riêng một bản private cho chính lý do này.
pub async fn fetch_dosing_calibration(
pool: &PgPool,
device_id: &str,
) -> Result<Option<DosingCalibration>, sqlx::Error> {
sqlx::query_as::<_, DosingCalibration>("SELECT * FROM dosing_calibration WHERE device_id = $1")
.bind(device_id)
.fetch_optional(pool)
.await
}

Step 4: Chạy test để xác nhận pass
Run: cd hydragrow-backend && cargo test fetch_dosing_calibration -- --test-threads=1
Expected: PASS (2 tests)

Step 5: Commit
git add hydragrow-backend/src/db/postgres.rs hydragrow-backend/src/db/tests/test_postgres.rs
git commit -m "feat(backend): add fetch_dosing_calibration for non-HTTP callers"

Task 3: Thêm pwm vào ActionCommandOutput
Files:
- Modify: hydragrow-backend/src/models/script.rs
- Modify: hydragrow-backend/src/services/script_engine.rs

Step 1: Viết failing test
// hydragrow-backend/src/services/script_engine.rs — thêm vào mod tests hiện có
#[test]
fn eval_action_command_reads_pwm_field() {
let engine = ScriptEngine::new();
let src = r#"
fn main(input) {
#{ action: "dose", pump: "ph_down", dose_ml: 3.0, pwm: 80 }
}
"#;
let ast = engine.compile(src).unwrap();
let input = ScriptActionInput {
ph: 8.0, ec: 1.5, temp: 25.0, water_level: 80.0,
phase: "Monitoring".into(), device_id: "d1".into(), timestamp_ms: 0,
};
let result = engine.eval_action_command(&ast, &input).unwrap().unwrap();
assert_eq!(result.pwm, Some(80));
}
#[test]
fn eval_action_command_pwm_is_none_when_absent() {
let engine = ScriptEngine::new();
let src = r#"fn main(input) { #{ action: "water_on", pump: "WATER_PUMP_IN", duration_sec: 10 } }"#;
let ast = engine.compile(src).unwrap();
let input = ScriptActionInput {
ph: 6.5, ec: 1.5, temp: 25.0, water_level: 80.0,
phase: "Monitoring".into(), device_id: "d1".into(), timestamp_ms: 0,
};
let result = engine.eval_action_command(&ast, &input).unwrap().unwrap();
assert_eq!(result.pwm, None);
}

Step 2: Chạy test để xác nhận fail
Run: cd hydragrow-backend && cargo test eval_action_command_reads_pwm eval_action_command_pwm_is_none -- --test-threads=1
Expected: FAIL biên dịch — ActionCommandOutput chưa có field pwm.

Step 3: Viết implementation
// hydragrow-backend/src/models/script.rs
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionCommandOutput {
pub action: String,
pub pump: Option<String>,
pub dose_ml: Option<f32>,
/// Chỉ có ý nghĩa khi action="dose" — % công suất bơm, dùng cùng
/// hydragrow_shared::dosing để quy đổi dose_ml → duration_sec.
pub pwm: Option<u32>,
pub duration_sec: Option<u64>,
}

// hydragrow-backend/src/services/script_engine.rs — trong eval_action_command,
// thêm dòng đọc "pwm" cạnh dòng đọc "duration_sec" hiện có:
let pwm = map.get("pwm").and_then(|v| v.clone().try_cast::<i64>()).map(|i| i as u32);
// ... và thêm `pwm` vào Ok(Some(ActionCommandOutput { action, pump, dose_ml, pwm, duration_sec }))

Step 4: Chạy test để xác nhận pass
Run: cd hydragrow-backend && cargo test eval_action_command -- --test-threads=1
Expected: PASS (toàn bộ test eval_action_command, bao gồm 2 test cũ từ Phase 0 + 2 test mới)

Step 5: Commit
git add hydragrow-backend/src/models/script.rs hydragrow-backend/src/services/script_engine.rs
git commit -m "feat(backend): ActionCommandOutput carries pwm for dose commands"

Task 4: Viết lại action_dispatch.rs — đúng wire format MqttCommandOut
Files:
- Modify: hydragrow-backend/src/services/action_dispatch.rs
- Modify: hydragrow-backend/src/mqtt/handlers/sensors.rs

Step 1: Viết failing test
// hydragrow-backend/src/services/action_dispatch.rs — thay toàn bộ mod tests cũ
#[cfg(test)]
mod tests {
use super::*;
fn limits() -> DoseSafetyLimits {
DoseSafetyLimits { max_dose_per_cycle_ml: 10.0, max_dose_per_hour_ml: 30.0, cooldown_sec: 60 }
}
fn calibration() -> DosingCalibration {
DosingCalibration {
device_id: "d1".to_string(),
pump_a_capacity_ml_per_sec: 1.0,
pump_b_capacity_ml_per_sec: 1.0,
pump_ph_up_capacity_ml_per_sec: 2.0,
pump_ph_down_capacity_ml_per_sec: 2.0,
..Default::default()
}
}
#[test]
fn non_dose_action_bypasses_safety_and_calibration() {
let output = ActionCommandOutput {
action: "water_on".into(), pump: Some("WATER_PUMP_IN".into()),
dose_ml: None, pwm: None, duration_sec: Some(10),
};
let decision = evaluate_action_safety(&output, &limits(), &[], 1_000, None, None).unwrap();
assert_eq!(decision, ActionSafetyDecision::Allow { duration_sec: Some(10) });
}
#[test]
fn dose_within_limits_computes_duration_from_calibration() {
let output = ActionCommandOutput {
action: "dose".into(), pump: Some("PH_DOWN".into()),
dose_ml: Some(4.0), pwm: Some(100), duration_sec: None,
};
let decision = evaluate_action_safety(&output, &limits(), &[], 1_000, None, Some(&calibration())).unwrap();
// capacity=2.0ml/s tại pwm=100% → 4.0ml / 2.0ml/s = 2s
assert_eq!(decision, ActionSafetyDecision::Allow { duration_sec: Some(2) });
}
#[test]
fn dose_exceeding_per_cycle_limit_is_blocked_before_calibration_lookup() {
let output = ActionCommandOutput {
action: "dose".into(), pump: Some("PH_DOWN".into()),
dose_ml: Some(50.0), pwm: Some(100), duration_sec: None,
};
// calibration=None: nếu code lỡ tính calibration TRƯỚC safety check, test này sẽ panic
// thay vì trả Block — thứ tự đúng là safety-check ml trước, calibration sau.
let decision = evaluate_action_safety(&output, &limits(), &[], 1_000, None, None).unwrap();
assert!(matches!(decision, ActionSafetyDecision::Block(_)));
}
#[test]
fn dose_missing_pwm_is_blocked_defensively() {
let output = ActionCommandOutput {
action: "dose".into(), pump: Some("PH_DOWN".into()),
dose_ml: Some(4.0), pwm: None, duration_sec: None,
};
let decision = evaluate_action_safety(&output, &limits(), &[], 1_000, None, Some(&calibration())).unwrap();
assert!(matches!(decision, ActionSafetyDecision::Block(_)));
}
#[test]
fn dose_with_unrecognized_pump_name_errors_instead_of_silently_using_zero_capacity() {
let output = ActionCommandOutput {
action: "dose".into(), pump: Some("NOT_A_PUMP".into()),
dose_ml: Some(4.0), pwm: Some(100), duration_sec: None,
};
let result = evaluate_action_safety(&output, &limits(), &[], 1_000, None, Some(&calibration()));
assert!(matches!(result, Err(ActionDispatchError::UnknownPump(_))));
}
#[test]
fn dose_without_calibration_row_errors_instead_of_dosing_blind() {
let output = ActionCommandOutput {
action: "dose".into(), pump: Some("PH_DOWN".into()),
dose_ml: Some(4.0), pwm: Some(100), duration_sec: None,
};
let result = evaluate_action_safety(&output, &limits(), &[], 1_000, None, None);
assert!(matches!(result, Err(ActionDispatchError::UnknownPump(_))));
}
}

Step 2: Chạy test để xác nhận fail
Run: cd hydragrow-backend && cargo test action_dispatch:: -- --test-threads=1
Expected: FAIL biên dịch — ActionSafetyDecision/ActionDispatchError đổi shape so với Phase 0.

Step 3: Viết implementation
// hydragrow-backend/src/services/action_dispatch.rs
//! Dispatch action_command script output ra MQTT. "dose" LUÔN đi qua
//! `hydragrow_shared::safety::check_dose` trước, sau đó quy đổi ml → duration_sec
//! bằng đúng công thức calibration mà `api/control.rs::validate_manual_dose_safety`
//! dùng để ước lượng ml cho lệnh manual — một công thức, hai nơi gọi, không lệch
//! nhau (module-rules/shared.md rule #2). Publish qua `MqttCommandOut` — đúng wire
//! format mà endpoint manual-control (`api::control::control_pump`) đang dùng thật,
//! KHÔNG dùng `services::command::CommandPayload` (thiếu field `pwm`/`dose_ml`).
use crate::AppState;
use crate::models::config::DosingCalibration;
use crate::models::script::ActionCommandOutput;
use hydragrow_shared::dosing::{
capacity_ml_per_sec_for_pump, ml_to_duration_sec, normalize_dosing_pump_name,
};
use hydragrow_shared::safety::{DoseSafetyLimits, DoseSafetyViolation, check_dose};
use hydragrow_shared::{MqttCommandOut, MqttCommandParams};

#[derive(Debug, PartialEq)]
pub enum ActionSafetyDecision {
Allow { duration_sec: Option<u64> },
Block(DoseSafetyViolation),
}

#[derive(Debug)]
pub enum ActionDispatchError {
Safety(DoseSafetyViolation),
UnknownPump(String),
UnknownAction(String),
Mqtt(anyhow::Error),
}

/// Thuần — test được không cần DB/MQTT. Thứ tự bắt buộc: check_dose (ml, không
/// phụ thuộc calibration) TRƯỚC, tra calibration để tính duration_sec SAU — để
/// một liều vượt ngưỡng bị chặn ngay cả khi calibration bị thiếu/lỗi.
pub fn evaluate_action_safety(
output: &ActionCommandOutput,
limits: &DoseSafetyLimits,
hourly_history_ml: &[(u64, f32)],
now_sec: u64,
last_dose_at_sec: Option<u64>,
calibration: Option<&DosingCalibration>,
) -> Result<ActionSafetyDecision, ActionDispatchError> {
if output.action != "dose" {
return Ok(ActionSafetyDecision::Allow { duration_sec: output.duration_sec });
}
let (Some(dose_ml), Some(pwm), Some(pump)) =
(output.dose_ml, output.pwm, output.pump.as_deref())
else {
return Ok(ActionSafetyDecision::Block(DoseSafetyViolation::ExceedsPerCycleLimit {
requested_ml: f32::INFINITY,
max_ml: limits.max_dose_per_cycle_ml,
}));
};
if let Err(violation) = check_dose(limits, hourly_history_ml, now_sec, last_dose_at_sec, dose_ml) {
return Ok(ActionSafetyDecision::Block(violation));
}
let Some(normalized_pump) = normalize_dosing_pump_name(pump) else {
return Err(ActionDispatchError::UnknownPump(pump.to_string()));
};
let Some(calibration) = calibration else {
return Err(ActionDispatchError::UnknownPump(format!(
"Không có dosing_calibration cho pump {normalized_pump}"
)));
};
let capacity = capacity_ml_per_sec_for_pump(
calibration.pump_a_capacity_ml_per_sec,
calibration.pump_b_capacity_ml_per_sec,
calibration.pump_ph_up_capacity_ml_per_sec,
calibration.pump_ph_down_capacity_ml_per_sec,
normalized_pump,
);
let Some(duration_sec) = ml_to_duration_sec(capacity, pwm, dose_ml) else {
return Err(ActionDispatchError::UnknownPump(format!(
"Không tính được duration_sec cho pump {normalized_pump} (capacity={capacity}, pwm={pwm})"
)));
};
Ok(ActionSafetyDecision::Allow { duration_sec: Some(duration_sec) })
}

/// Entry point gọi từ MQTT handler.
pub async fn dispatch_action_command(
app_state: &AppState,
device_id: &str,
output: ActionCommandOutput,
limits: &DoseSafetyLimits,
hourly_history_ml: &[(u64, f32)],
now_sec: u64,
last_dose_at_sec: Option<u64>,
calibration: Option<&DosingCalibration>,
) -> Result<(), ActionDispatchError> {
if output.action == "emergency_stop" {
return crate::services::command::trigger_emergency_stop(app_state, device_id)
.await
.map_err(ActionDispatchError::Mqtt);
}
let decision = evaluate_action_safety(&output, limits, hourly_history_ml, now_sec, last_dose_at_sec, calibration)?;
let duration_sec = match decision {
ActionSafetyDecision::Block(violation) => return Err(ActionDispatchError::Safety(violation)),
ActionSafetyDecision::Allow { duration_sec } => duration_sec,
};
let mqtt_action = match output.action.as_str() {
"dose" => "set_pwm",
"water_on" => "pump_on",
"water_off" => "pump_off",
other => return Err(ActionDispatchError::UnknownAction(other.to_string())),
};
let command = MqttCommandOut {
target: "all".to_string(), // = giá trị mặc định của resolve_control_target(None)
action: mqtt_action.to_string(),
params: Some(MqttCommandParams {
pump_id: output.pump.clone(),
duration_sec,
pwm: if output.action == "dose" { output.pwm } else { None },
state: None,
ota_url: None,
candidates: None,
}),
ts: None,
nonce: None,
signature: None,
};
crate::api::mqtt_utils::publish_command(app_state, device_id, &command)
.await
.map_err(ActionDispatchError::Mqtt)
}

Cập nhật call site trong sensors.rs:
// hydragrow-backend/src/mqtt/handlers/sensors.rs
let action_scripts = app_state.script_cache.get_action_command_scripts(&device_id).await;
if !action_scripts.is_empty() {
if let Ok(safety_config) = crate::db::postgres::get_safety_config(&app_state.pg_pool, &device_id).await {
let calibration = crate::db::postgres::fetch_dosing_calibration(&app_state.pg_pool, &device_id)
.await
.unwrap_or(None);
let limits = hydragrow_shared::safety::DoseSafetyLimits {
max_dose_per_cycle_ml: safety_config.max_dose_per_cycle,
max_dose_per_hour_ml: safety_config.max_dose_per_hour,
cooldown_sec: safety_config.cooldown_sec as u64,
};
let action_input = crate::models::script::ScriptActionInput {
ph: incoming.ph,
ec: incoming.ec,
temp: incoming.temp,
water_level: incoming.water_level,
phase: "Monitoring".to_string(), // TODO(follow-up, ghi nhận từ Phase 0): lấy phase thật từ device_states cache
device_id: device_id.clone(),
timestamp_ms: chrono::Utc::now().timestamp_millis(),
};
let engine = std::sync::Arc::new(crate::services::script_engine::ScriptEngine::new());
let now_sec = (action_input.timestamp_ms / 1000) as u64;
for script in &action_scripts {
if let Ok(Some(output)) = engine.eval_action_command(&script.ast, &action_input) {
if let Err(err) = crate::services::action_dispatch::dispatch_action_command(
&app_state, &device_id, output, &limits, &[], now_sec, None, calibration.as_ref(),
)
.await
{
tracing::warn!(
script_id = %script.id, device_id, error = ?err,
"action_command bị chặn hoặc lỗi khi dispatch"
);
}
}
}
}
}

Step 4: Chạy test để xác nhận pass
Run: cd hydragrow-backend && cargo test action_dispatch:: -- --test-threads=1
Expected: PASS (7 tests)

Step 5: Commit
git add hydragrow-backend/src/services/action_dispatch.rs hydragrow-backend/src/mqtt/handlers/sensors.rs
git commit -m "fix(backend): action_dispatch publishes real MqttCommandOut wire format"

Task 5: Frontend IR — thêm kind: 'action_command' + 4 Action schema mới
Files:
- Modify: hydragrow-frontend/src/lib/automation/ir.ts
- Modify: hydragrow-frontend/src/types/automation.ts
- Modify: hydragrow-frontend/src/lib/automation/ir.test.ts

Step 1: Viết failing test
// hydragrow-frontend/src/lib/automation/ir.test.ts — thêm vào cuối
import { AutomationIrSchema } from './ir';
describe('action_command IR', () => {
it('accepts a valid dose action', () => {
const result = AutomationIrSchema.safeParse({
kind: 'action_command',
trigger: { type: 'sensor' },
conditions: [{ sensor: 'ph', operator: '>', value: 7.5 }],
actions: [{ type: 'dose', pump: 'PH_DOWN', doseMl: 3, pwm: 80 }],
nodes: [],
edges: [],
});
expect(result.success).toBe(true);
});
it('accepts a valid water_on action', () => {
const result = AutomationIrSchema.safeParse({
kind: 'action_command',
trigger: { type: 'sensor' },
conditions: [{ sensor: 'water_level', operator: '<', value: 20 }],
actions: [{ type: 'water_on', pump: 'WATER_PUMP_IN', durationSec: 30 }],
nodes: [],
edges: [],
});
expect(result.success).toBe(true);
});
it('accepts a valid emergency_stop action with no conditions', () => {
const result = AutomationIrSchema.safeParse({
kind: 'action_command',
trigger: { type: 'sensor' },
conditions: [{ sensor: 'ph', operator: '>', value: 9.0 }],
actions: [{ type: 'emergency_stop' }],
nodes: [],
edges: [],
});
expect(result.success).toBe(true);
});
it('rejects an alert action under kind=action_command', () => {
const result = AutomationIrSchema.safeParse({
kind: 'action_command',
trigger: { type: 'sensor' },
conditions: [],
actions: [{ type: 'alert', level: 'warning', message: 'x' }],
nodes: [],
edges: [],
});
expect(result.success).toBe(false);
});
it('rejects pwm outside 1-100', () => {
const result = AutomationIrSchema.safeParse({
kind: 'action_command',
trigger: { type: 'sensor' },
conditions: [],
actions: [{ type: 'dose', pump: 'PH_DOWN', doseMl: 3, pwm: 150 }],
nodes: [],
edges: [],
});
expect(result.success).toBe(false);
});
});

Step 2: Chạy test để xác nhận fail
Run: cd hydragrow-frontend && npx vitest run ir.test
Expected: FAIL — kind: 'action_command' bị AutomationIrSchema từ chối (chưa nằm trong enum).

Step 3: Viết implementation
// hydragrow-frontend/src/lib/automation/ir.ts
// 1. Mở rộng kind enum (sửa dòng khai báo cũ `kind: z.enum(['alert', 'recipe_override'])`)
export const AutomationKindSchema = z.enum(['alert', 'recipe_override', 'action_command']);
// 2. Thêm các schema pump/action mới (đặt cạnh AlertActionSchema/StageOverrideActionSchema hiện có)
export const DosingPumpSchema = z.enum(['PUMP_A', 'PUMP_B', 'PH_UP', 'PH_DOWN']);
export const WaterPumpSchema = z.enum(['WATER_PUMP_IN', 'WATER_PUMP_OUT', 'MIST_VALVE', 'OSAKA_PUMP']);
export const DoseActionSchema = z.object({
type: z.literal('dose'),
pump: DosingPumpSchema,
doseMl: z.number().positive(),
pwm: z.number().int().min(1).max(100),
});
export const WaterOnActionSchema = z.object({
type: z.literal('water_on'),
pump: WaterPumpSchema,
durationSec: z.number().int().positive(),
});
export const WaterOffActionSchema = z.object({
type: z.literal('water_off'),
pump: WaterPumpSchema,
});
export const EmergencyStopActionSchema = z.object({
type: z.literal('emergency_stop'),
});
// 3. Mở rộng discriminated union (sửa khai báo ActionSchema hiện có để thêm 4 nhánh)
export const ActionSchema = z.discriminatedUnion('type', [
AlertActionSchema,
StageOverrideActionSchema,
DoseActionSchema,
WaterOnActionSchema,
WaterOffActionSchema,
EmergencyStopActionSchema,
]);
// 4. Cập nhật .refine() ràng buộc "actions phải khớp kind" trong AutomationIrSchema:
// (sửa hàm refine hiện có, thêm nhánh action_command)
// if (ir.kind === 'alert') return ir.actions.every((a) => a.type === 'alert');
// if (ir.kind === 'recipe_override') return ir.actions.every((a) => a.type === 'advance_stage');
// return ir.actions.every((a) => ['dose', 'water_on', 'water_off', 'emergency_stop'].includes(a.type));

// hydragrow-frontend/src/types/automation.ts
export interface UserScript {
id: string;
device_id: string;
kind: 'alert' | 'recipe_override' | 'action_command';
name: string;
source: string;
enabled: boolean;
ir_json: AutomationIr | null;
created_at: string;
updated_at: string;
}
export interface UpsertScriptRequest {
kind: 'alert' | 'recipe_override' | 'action_command';
name: string;
source: string;
enabled?: boolean;
ir_json?: AutomationIr;
}

Step 4: Chạy test để xác nhận pass
Run: cd hydragrow-frontend && npx tsc --noEmit && npx vitest run ir.test
Expected: PASS.

Step 5: Commit
git add hydragrow-frontend/src/lib/automation/ir.ts hydragrow-frontend/src/lib/automation/ir.test.ts \
hydragrow-frontend/src/types/automation.ts
git commit -m "feat(frontend): add action_command kind + dose/water/emergency_stop action schemas"

Task 6: compileToRhai.ts — biên dịch 4 action mới ra Rhai
Files:
- Modify: hydragrow-frontend/src/lib/automation/compileToRhai.ts
- Modify: hydragrow-frontend/src/lib/automation/compileToRhai.test.ts

Step 1: Viết failing test
// hydragrow-frontend/src/lib/automation/compileToRhai.test.ts — thêm vào cuối
describe('action_command compilation', () => {
it('compiles a dose action with snake_case keys matching eval_action_command', () => {
const source = compileToRhai({
kind: 'action_command',
trigger: { type: 'sensor' },
conditions: [{ sensor: 'ph', operator: '>', value: 7.5 }],
actions: [{ type: 'dose', pump: 'PH_DOWN', doseMl: 3, pwm: 80 }],
nodes: [],
edges: [],
});
expect(source).toContain('"action": "dose"');
expect(source).toContain('"pump": "PH_DOWN"');
expect(source).toContain('"dose_ml": 3');
expect(source).toContain('"pwm": 80');
});
it('compiles a water_off action without a duration_sec key', () => {
const source = compileToRhai({
kind: 'action_command',
trigger: { type: 'sensor' },
conditions: [],
actions: [{ type: 'water_off', pump: 'WATER_PUMP_IN' }],
nodes: [],
edges: [],
});
expect(source).toContain('"action": "water_off"');
expect(source).not.toContain('duration_sec');
});
it('compiles emergency_stop with no other fields', () => {
const source = compileToRhai({
kind: 'action_command',
trigger: { type: 'sensor' },
conditions: [{ sensor: 'ph', operator: '>', value: 9.0 }],
actions: [{ type: 'emergency_stop' }],
nodes: [],
edges: [],
});
expect(source).toContain('"action": "emergency_stop"');
});
});

Step 2: Chạy test để xác nhận fail
Run: cd hydragrow-frontend && npx vitest run compileToRhai
Expected: FAIL — actionToRhaiMap chưa xử lý 4 type mới

Step 3: Viết implementation
// hydragrow-frontend/src/lib/automation/compileToRhai.ts
// Thay toàn bộ hàm actionToRhaiMap bằng bản switch đầy đủ:
function actionToRhaiMap(action: Action): string {
switch (action.type) {
case 'alert': {
const title = action.title ?? action.message;
return [
'#{',
`  "level": "${rhaiString(action.level)}",`,
`  "title": "${rhaiString(title)}",`,
`  "message": "${rhaiString(action.message)}"`,
'}',
].join('\n  ');
}
case 'advance_stage': {
const offsetExpr =
action.targetStageOffset === 0
? 'input.stage_index'
: action.targetStageOffset > 0
? `input.stage_index + ${action.targetStageOffset}`
: `input.stage_index - ${Math.abs(action.targetStageOffset)}`;
return [
'#{',
`  "target_stage_index": ${offsetExpr},`,
`  "reason": "${rhaiString(action.reason)}"`,
'}',
].join('\n  ');
}
case 'dose':
return [
'#{',
`  "action": "dose",`,
`  "pump": "${rhaiString(action.pump)}",`,
`  "dose_ml": ${action.doseMl},`,
`  "pwm": ${action.pwm}`,
'}',
].join('\n  ');
case 'water_on':
return [
'#{',
`  "action": "water_on",`,
`  "pump": "${rhaiString(action.pump)}",`,
`  "duration_sec": ${action.durationSec}`,
'}',
].join('\n  ');
case 'water_off':
return [
'#{',
`  "action": "water_off",`,
`  "pump": "${rhaiString(action.pump)}"`,
'}',
].join('\n  ');
case 'emergency_stop':
return '#{ "action": "emergency_stop" }';
}
}

Step 4: Chạy test để xác nhận pass
Run: cd hydragrow-frontend && npx tsc --noEmit && npx vitest run compileToRhai
Expected: PASS.

Step 5: Commit
git add hydragrow-frontend/src/lib/automation/compileToRhai.ts hydragrow-frontend/src/lib/automation/compileToRhai.test.ts
git commit -m "feat(frontend): compile dose/water/emergency_stop actions to Rhai"

Task 7: Block Blockly mới — Dose / Water / Emergency Stop
Files:
- Modify: hydragrow-frontend/src/components/automation/blockly/blocks.ts
- Modify: hydragrow-frontend/src/components/automation/blockly/extractIr.ts
- Modify: hydragrow-frontend/src/components/automation/blockly/hydrateIr.ts
- Modify: hydragrow-frontend/src/components/automation/blockly/hydrateIr.test.ts

Step 1: Viết failing test
// hydragrow-frontend/src/components/automation/blockly/hydrateIr.test.ts — thêm vào describe hiện có
it('round-trips a dose action', () => {
hydrateWorkspace(workspace, [], [{ type: 'dose', pump: 'PH_DOWN', doseMl: 3, pwm: 80 }]);
expect(extractActions(workspace)).toEqual([{ type: 'dose', pump: 'PH_DOWN', doseMl: 3, pwm: 80 }]);
});
it('round-trips a water_on action', () => {
hydrateWorkspace(workspace, [], [{ type: 'water_on', pump: 'WATER_PUMP_IN', durationSec: 30 }]);
expect(extractActions(workspace)).toEqual([{ type: 'water_on', pump: 'WATER_PUMP_IN', durationSec: 30 }]);
});
it('round-trips a water_off action', () => {
hydrateWorkspace(workspace, [], [{ type: 'water_off', pump: 'MIST_VALVE' }]);
expect(extractActions(workspace)).toEqual([{ type: 'water_off', pump: 'MIST_VALVE' }]);
});
it('round-trips an emergency_stop action', () => {
hydrateWorkspace(workspace, [], [{ type: 'emergency_stop' }]);
expect(extractActions(workspace)).toEqual([{ type: 'emergency_stop' }]);
});

Step 2: Chạy test để xác nhận fail
Run: cd hydragrow-frontend && npx vitest run hydrateIr
Expected: FAIL

Step 3: Viết implementation
// hydragrow-frontend/src/components/automation/blockly/blocks.ts
Blockly.Blocks['hydragrow_dose_action'] = {
init(this: Blockly.Block) {
this.appendDummyInput()
.appendField('Dose')
.appendField(
new Blockly.FieldDropdown([
['PUMP_A', 'PUMP_A'],
['PUMP_B', 'PUMP_B'],
['PH_UP', 'PH_UP'],
['PH_DOWN', 'PH_DOWN'],
]),
'PUMP',
);
this.appendDummyInput()
.appendField('ml')
.appendField(new Blockly.FieldNumber(1, 0), 'DOSE_ML')
.appendField('PWM %')
.appendField(new Blockly.FieldNumber(100, 1, 100), 'PWM');
this.setPreviousStatement(true, 'action');
this.setColour(0);
this.setTooltip('Bơm một liều dung dịch — luôn đi qua safety gate ở backend trước khi publish.');
},
};
Blockly.Blocks['hydragrow_water_action'] = {
init(this: Blockly.Block) {
this.appendDummyInput()
.appendField('Water')
.appendField(
new Blockly.FieldDropdown([
['WATER_PUMP_IN', 'WATER_PUMP_IN'],
['WATER_PUMP_OUT', 'WATER_PUMP_OUT'],
['MIST_VALVE', 'MIST_VALVE'],
['OSAKA_PUMP', 'OSAKA_PUMP'],
]),
'PUMP',
)
.appendField(new Blockly.FieldDropdown([['on', 'on'], ['off', 'off']]), 'STATE');
this.appendDummyInput()
.appendField('giây (chỉ dùng khi bật)')
.appendField(new Blockly.FieldNumber(10, 0), 'DURATION_SEC');
this.setPreviousStatement(true, 'action');
this.setColour(200);
this.setTooltip('Bật/tắt bơm nước hoặc van tuần hoàn.');
},
};
Blockly.Blocks['hydragrow_emergency_stop_action'] = {
init(this: Blockly.Block) {
this.appendDummyInput().appendField('EMERGENCY STOP — dừng mọi actor');
this.setPreviousStatement(true, 'action');
this.setColour(0);
this.setTooltip('Publish lệnh dừng khẩn cấp cho toàn bộ thiết bị.');
},
};

// hydragrow-frontend/src/components/automation/blockly/extractIr.ts
const doses: Action[] = workspace.getBlocksByType('hydragrow_dose_action', false).map((block) => ({
type: 'dose' as const,
pump: block.getFieldValue('PUMP') as 'PUMP_A' | 'PUMP_B' | 'PH_UP' | 'PH_DOWN',
doseMl: Number(block.getFieldValue('DOSE_ML')),
pwm: Number(block.getFieldValue('PWM')),
}));
const waters: Action[] = workspace.getBlocksByType('hydragrow_water_action', false).map((block) => {
const pump = block.getFieldValue('PUMP') as 'WATER_PUMP_IN' | 'WATER_PUMP_OUT' | 'MIST_VALVE' | 'OSAKA_PUMP';
if (block.getFieldValue('STATE') === 'on') {
return { type: 'water_on' as const, pump, durationSec: Number(block.getFieldValue('DURATION_SEC')) };
}
return { type: 'water_off' as const, pump };
});
const emergencyStops: Action[] = workspace
.getBlocksByType('hydragrow_emergency_stop_action', false)
.map(() => ({ type: 'emergency_stop' as const }));
// nối vào return statement hiện có: return [...alerts, ...advances, ...doses, ...waters, ...emergencyStops];

// hydragrow-frontend/src/components/automation/blockly/hydrateIr.ts
} else if (action.type === 'dose') {
const block = workspace.newBlock('hydragrow_dose_action');
block.setFieldValue(action.pump, 'PUMP');
block.setFieldValue(String(action.doseMl), 'DOSE_ML');
block.setFieldValue(String(action.pwm), 'PWM');
placeAndChain(block);
} else if (action.type === 'water_on' || action.type === 'water_off') {
const block = workspace.newBlock('hydragrow_water_action');
block.setFieldValue(action.pump, 'PUMP');
block.setFieldValue(action.type === 'water_on' ? 'on' : 'off', 'STATE');
if (action.type === 'water_on') {
block.setFieldValue(String(action.durationSec), 'DURATION_SEC');
}
placeAndChain(block);
} else if (action.type === 'emergency_stop') {
const block = workspace.newBlock('hydragrow_emergency_stop_action');
placeAndChain(block);
}

Step 4: Chạy test để xác nhận pass
Run: cd hydragrow-frontend && npx vitest run hydrateIr extractIr
Expected: PASS toàn bộ

Step 5: Commit
git add hydragrow-frontend/src/components/automation/blockly/blocks.ts \
hydragrow-frontend/src/components/automation/blockly/extractIr.ts \
hydragrow-frontend/src/components/automation/blockly/hydrateIr.ts \
hydragrow-frontend/src/components/automation/blockly/hydrateIr.test.ts
git commit -m "feat(frontend): add Dose/Water/EmergencyStop Blockly blocks with round-trip hydrate"

Task 8: Toolbox + UI chọn kind action_command
Files:
- Modify: hydragrow-frontend/src/components/automation/BlockLogicEditor.tsx
- Modify: hydragrow-frontend/src/components/automation/FlowDetailDrawer.tsx
- Modify: hydragrow-frontend/src/components/automation/FlowDetailDrawer.test.tsx

Step 1: Viết failing test
// hydragrow-frontend/src/components/automation/FlowDetailDrawer.test.tsx — thêm vào describe buildAutomationIr
it('builds a sensor-triggered IR for kind=action_command', () => {
const ir = buildAutomationIr('action_command', {
conditions: [{ sensor: 'ph', operator: '>', value: 8.0 }],
actions: [{ type: 'dose', pump: 'PH_DOWN', doseMl: 3, pwm: 80 }],
});
expect(ir.trigger).toEqual({ type: 'sensor' });
});

Step 2: Chạy test để xác nhận fail
Run: cd hydragrow-frontend && npx vitest run FlowDetailDrawer
Expected: FAIL biên dịch

Step 3: Viết implementation
// hydragrow-frontend/src/components/automation/FlowDetailDrawer.tsx
const TRIGGER_FOR_KIND: Record<AutomationIr['kind'], AutomationIr['trigger']> = {
alert: { type: 'sensor' },
recipe_override: { type: 'fsm' },
action_command: { type: 'sensor' }, // giống alert — eval trên mỗi sensor message (xem sensors.rs)
};
Thêm option vào <select> chọn kind (trong cùng file, JSX phần render):
<select
className="rounded border px-2 py-1 text-sm"
value={kind}
onChange={(e) => setKind(e.target.value as AutomationIr['kind'])}
>
<option value="alert">Alert</option>
<option value="recipe_override">Recipe Override</option>
<option value="action_command">Action Command</option>
</select>

Mở rộng toolboxFor trong BlockLogicEditor.tsx:
// hydragrow-frontend/src/components/automation/BlockLogicEditor.tsx
function toolboxFor(kind: AutomationIr['kind']) {
if (kind === 'action_command') {
return {
kind: 'flyoutToolbox',
contents: [
{ kind: 'block', type: 'hydragrow_sensor_condition' },
{ kind: 'block', type: 'hydragrow_dose_action' },
{ kind: 'block', type: 'hydragrow_water_action' },
{ kind: 'block', type: 'hydragrow_emergency_stop_action' },
],
};
}
return {
kind: 'flyoutToolbox',
contents: [
{ kind: 'block', type: 'hydragrow_sensor_condition' },
{ kind: 'block', type: kind === 'alert' ? 'hydragrow_alert_action' : 'hydragrow_advance_stage_action' },
],
};
}
Và điều kiện chọn field set cho condition dropdown:
registerHydragrowBlocks(kind === 'recipe_override' ? FSM_FIELDS : SENSOR_FIELDS);

Step 4: Chạy test để xác nhận pass
Run: cd hydragrow-frontend && npx tsc --noEmit && npx vitest run FlowDetailDrawer
Expected: PASS.

Step 5: Chạy toàn bộ test suite automation để xác nhận không regression
Run: cd hydragrow-frontend && npx vitest run src/components/automation src/lib/automation src/hooks/useFlowCanvas
Expected: PASS toàn bộ (Phase 0 + Phase 1).

Step 6: Commit
git add hydragrow-frontend/src/components/automation/BlockLogicEditor.tsx \
hydragrow-frontend/src/components/automation/FlowDetailDrawer.tsx \
hydragrow-frontend/src/components/automation/FlowDetailDrawer.test.tsx
git commit -m "feat(frontend): wire action_command kind into toolbox and Flow detail drawer"
