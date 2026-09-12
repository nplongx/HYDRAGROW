# Diagnostic Hestia + Local Qwen3 Pipeline Implementation Plan

Goal: Complete the HYDRAGROW diagnostic vertical slice from cached controller Hestia state through GET /api/health
hestia, worker parsing/triggering, local Qwen3/llama.cpp diagnosis, and verified production behavior.

Architecture: Keep Hestia calculation on the controller and reuse the backend's existing AppState.device_states cache rather
than introducing a new persistence path. Expose a fleet endpoint using the existing health:read authentication convention and
the existing API envelope, then make BackendClient consume that envelope. Keep the diagnostic model read-only: it may query
historical/health data through existing supervisor-query tools, but it must never receive actuator-control capabilities.

Tech Stack: Rust, Actix-web, Tokio, Serde/serde_json, reqwest, sqlx/PostgreSQL, MQTT/rumqttc, llama.cpp OpenAI-compatible /v1
chat/completions, Qwen3-0.6B GGUF.

Spec: Current diagnostic-worker/Hestia requirements in the repository and the production failure reported as GET /api/health
hestia returned 404 Not Found.

## Global Constraints

- Preserve the existing health:read scope and API-key authentication path; do not introduce a second auth mechanism.
- Reuse DeviceHealthSnapshot and HestiaAssessment; do not invent a parallel Hestia schema.
- Missing Hestia data remains missing; never synthesize a healthy/default assessment.
- Keep the diagnostic LLM read-only; never add pump/valve/dosing/FSM-control tools.
- Keep local provider support for local, llama, and llama.cpp; preserve openrouter support.
- Never hard-code or log API keys, model secrets, or other credentials.
- Prefer minimal changes over refactoring unrelated code.
- Every task ends with a focused test/verification cycle and a small commit.

## File Structure Map

Backend:

- hydragrow-backend/src/api/health_topics.rs — existing fleet health API; add the fleet Hestia response/helper
  and route here to preserve the existing health API boundary.
- hydragrow-backend/src/main.rs — route registration is already
  wired through api::health_topics::init_fleet_routes; modify only if inspection shows registration differs from the
  expected /api scope.

Shared types:

- hydragrow-shared/src/telemetry/health.rs — canonical DeviceHealthSnapshot; no schema rewrite expected.
- hydragrow-shared/src/hestia.rs — canonical HestiaAssessment/HestiaState; no semantic rewrite expected.

Diagnostic worker:

- hydragrow-diagnostic-worker/src/backend_client.rs — parse the backend's {status,data} API
  envelope for fleet Hestia and retain a focused mock test.
- hydragrow-diagnostic-worker/src/tick.rs — validate trigger
  behavior against the real fleet-Hestia shape; only change if contract tests demonstrate a mismatch.
- hydragrow-diagnostic-worker/src/local_model.rs — verify/strengthen llama.cpp response/tool-roundtrip handling only where tests expose a real gap.
- hydragrow-diagnostic-worker/src/diagnostic_model.rs — existing strict diagnosis parser/fallback remains the validation boundary.

## Task 1: Lock Down the Backend Hestia Response Contract

Files:

- Modify: hydragrow-backend/src/api/health_topics.rs
- Test: hydragrow-backend/src/api/health_topics.rs

Interfaces:

- Consumes: AppState.device_states: Arc<RwLock<HashMap<String, String>>>,
  DeviceHealthSnapshot, HestiaAssessment, existing auth_or_forbidden/health:read convention.
- Produces: a pure helper that converts cached serialized snapshots into HashMap<String, HestiaAssessment>, plus GET /health/hestia
  under the existing fleet health route configuration. The HTTP response shape must be { "status": "success", "data": { ... } }.

### Step 1: Write the failing helper tests

Add a pure helper with an explicit signature before implementing it:

```rust
pub fn extract_hestia_by_device(states: &HashMap<String, String>) -> HashMap<String, HestiaAssessment>;
```

Add these cases in the existing #[cfg(test)] module. Fixture values MUST match the canonical schemas exactly:
HestiaState uses SCREAMING_SNAKE_CASE ("WARNING"), HestiaTrendDirection uses snake_case ("degrading", "stable"),
and HestiaAxesAssessment requires all four axes (ec, ph, water_level, temp), each with
comfort/weight/trend/trend_factor/action_factor:

```rust
#[test]
fn extract_hestia_by_device_returns_only_snapshots_with_hestia() {
    let warning = serde_json::json!({
        "device_id": "dev-1",
        "free_heap": 100,
        "uptime_sec": 10,
        "rssi": -40,
        "health_score_percent": 70,
        "fsm_state_display": "Running",
        "log_drop_count": 0,
        "firmware_version": "v1",
        "kalman_confidence": null,
        "matrix_update_count": 1,
        "matrix_is_warm": true,
        "hestia": {
            "score": 65.0,
            "state": "WARNING",
            "confidence": 0.8,
            "axes": {
                "ec": {"comfort": 0.5, "weight": 0.35, "trend": "degrading", "trend_factor": 1.1, "action_factor": 1.0},
                "ph": {"comfort": 1.0, "weight": 0.3, "trend": "stable", "trend_factor": 1.0, "action_factor": 1.0},
                "water_level": {"comfort": 1.0, "weight": 0.2, "trend": "stable", "trend_factor": 1.0, "action_factor": 1.0},
                "temp": {"comfort": 1.0, "weight": 0.15, "trend": "stable", "trend_factor": 1.0, "action_factor": 1.0}
            },
            "reasons": ["ec_out_of_range"]
        },
        "timestamp_ms": 1234
    });
    let no_hestia = serde_json::json!({
        "device_id": "dev-2",
        "free_heap": 100,
        "uptime_sec": 10,
        "rssi": -40,
        "health_score_percent": 100,
        "fsm_state_display": "Running",
        "log_drop_count": 0,
        "firmware_version": "v1",
        "kalman_confidence": null,
        "matrix_update_count": 1,
        "matrix_is_warm": false,
        "hestia": null,
        "timestamp_ms": 1234
    });

    let states = HashMap::from([
        ("dev-1".to_string(), warning.to_string()),
        ("dev-2".to_string(), no_hestia.to_string()),
    ]);

    let result = extract_hestia_by_device(&states);

    assert_eq!(result.len(), 1);
    assert_eq!(result["dev-1"].state, HestiaState::Warning);
    assert_eq!(result["dev-1"].reasons, vec!["ec_out_of_range"]);
}
```

Also add an invalid JSON case and a structurally invalid snapshot case; both must be skipped rather than panic:

```rust
#[test]
fn extract_hestia_by_device_skips_invalid_cached_state() {
    let states = HashMap::from([("broken".to_string(), "not-json".to_string())]);
    assert!(extract_hestia_by_device(&states).is_empty());
}
```

### Step 2: Run the focused tests to prove they fail

Run:

```bash
cargo test -p hydragrow-backend api::health_topics::tests::extract_hestia_by_device -- --nocapture
```

Expected before implementation: compilation failure because extract_hestia_by_device does not exist yet.

### Step 3: Implement the minimal pure helper

Add imports:

```rust
use hydragrow_shared::{hestia::HestiaAssessment, telemetry::health::DeviceHealthSnapshot};
```

Implement exactly this behavior:

```rust
pub fn extract_hestia_by_device(
    states: &HashMap<String, String>,
) -> HashMap<String, HestiaAssessment> {
    states
        .iter()
        .filter_map(|(device_id, raw)| {
            let snapshot = serde_json::from_str::<DeviceHealthSnapshot>(raw).ok()?;
            let hestia = snapshot.hestia?;
            Some((device_id.clone(), hestia))
        })
        .collect()
}
```

Do not add fallback values for failed deserialization or None Hestia.

### Step 4: Run the focused tests to prove they pass

Run:

```bash
cargo test -p hydragrow-backend api::health_topics::tests::extract_hestia_by_device -- --nocapture
```

Expected: PASS for all new helper tests.

### Step 5: Commit the isolated helper

```bash
git add hydragrow-backend/src/api/health_topics.rs
git commit -m "test: define fleet hestia extraction contract"
```

## Task 2: Add GET /api/health/hestia

Files:

- Modify: hydragrow-backend/src/api/health_topics.rs
- Modify: hydragrow-backend/src/main.rs only if route-scope inspection proves the /api prefix is not already supplied
- Test: hydragrow-backend/src/api/health_topics.rs

Interfaces:

- Consumes: extract_hestia_by_device, existing request auth extensions, AppState.
- Produces: get_all_hestia(req, app_state) and fleet route registration at /health/hestia within the same scope that currently
  makes /api/health/topics reachable as /api/health/topics.

### Step 1: Write the failing response-shape test

Add a pure serialization test for the exact API envelope:

```rust
#[test]
fn hestia_response_uses_existing_success_envelope() {
    let data = HashMap::<String, HestiaAssessment>::new();
    let body = serde_json::json!({"status": "success", "data": data});
    assert_eq!(body["status"], "success");
    assert!(body["data"].is_object());
}
```

Then add an Actix handler test following the repository's existing API test pattern if one exists in neighboring modules. The test
must verify that a request with an AuthContext containing health:read produces HTTP 200 and JSON with status=success;
use the repository's real middleware/extractor conventions rather than constructing a fake auth mechanism.

### Step 2: Run the focused route tests to prove the route is missing

Run:

```bash
cargo test -p hydragrow-backend api::health_topics::tests -- --nocapture
```

Expected: the new route test fails before implementation.

### Step 3: Implement the handler

Add:

```rust
pub async fn get_all_hestia(
    req: HttpRequest,
    app_state: web::Data<AppState>,
) -> impl Responder {
    if let Err(resp) = auth_or_forbidden(&req) {
        return resp;
    }

    let states = app_state.device_states.read().await;
    let data = extract_hestia_by_device(&states);

    HttpResponse::Ok().json(json!({
        "status": "success",
        "data": data,
    }))
}
```

Do not add database calls: the controller status handler already caches DeviceHealthSnapshot in device_states, and
DeviceHealthSnapshot contains the optional Hestia assessment.

Register the route alongside the existing fleet route:

```rust
pub fn init_fleet_routes(cfg: &mut web::ServiceConfig) {
    cfg.route("/health/topics", web::get().to(get_all_health_topics));
    cfg.route("/health/hestia", web::get().to(get_all_hestia));
}
```

Confirm from main.rs that api::health_topics::init_fleet_routes is mounted inside the /api scope. Do not duplicate
or relocate the scope.

### Step 4: Run backend tests and compile check

Run:

```bash
cargo test -p hydragrow-backend api::health_topics::tests -- --nocapture
cargo check -p hydragrow-backend
```

Expected: all focused health-topic/Hestia tests PASS and backend compilation succeeds.

### Step 5: Commit the API endpoint

```bash
git add hydragrow-backend/src/api/health_topics.rs hydragrow-backend/src/main.rs
git commit -m "feat: expose fleet hestia health endpoint"
```

## Task 3: Fix the Diagnostic Worker Fleet-Hestia Envelope Contract

Files:

- Modify: hydragrow-diagnostic-worker/src/backend_client.rs
- Test: hydragrow-diagnostic-worker/src/backend_client.rs

Interfaces:

- Consumes: backend JSON { "status": "success", "data": {device_id: HestiaAssessment JSON} }.
- Produces: unchanged trait method DiagnosisBackend::get_fleet_hestia() -> anyhow::Result<HashMap<String, serde_json::Value>>, returning only the data map.

### Step 1: Write the failing mock-response test

Replace/add the mock body so it matches the real health API envelope:

```rust
.with_body(
    r#"{
        "status":"success",
        "data":{
            "dev-1":{
                "score":65.0,
                "state":"WARNING",
                "confidence":0.8,
                "axes":{},
                "reasons":["ec_out_of_range"]
            }
        }
    }"#,
)
```

Keep the assertion:

```rust
assert_eq!(fleet["dev-1"]["state"], "WARNING");
```

Add a second test proving a non-success envelope is rejected even if HTTP status is 200:

```rust
#[tokio::test]
async fn get_fleet_hestia_rejects_api_error_envelope() {
    let mut server = mockito::Server::new_async().await;
    let _m = server
        .mock("GET", "/api/health/hestia")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"status":"error","data":{}}"#)
        .create_async()
        .await;

    let client = BackendClient::new(server.url(), "svc_test".to_string());
    assert!(client.get_fleet_hestia().await.is_err());
}
```

### Step 2: Run the worker client tests to prove the contract currently fails

Run:

```bash
cargo test -p hydragrow-diagnostic-worker backend_client::tests::get_fleet_hestia -- --nocapture
```

Expected: the envelope test fails because the current client deserializes the whole response directly into HashMap.

### Step 3: Implement a typed success envelope

Add near CreateEventBody:

```rust
#[derive(serde::Deserialize)]
struct ApiEnvelope<T> {
    status: String,
    data: T,
}
```

Change get_fleet_hestia to:

```rust
let envelope: ApiEnvelope<HashMap<String, serde_json::Value>> = resp.json().await?;
if envelope.status != "success" {
    anyhow::bail!("GET /api/health/hestia returned API status {}", envelope.status);
}
Ok(envelope.data)
```

Do not weaken HTTP status checking; preserve the existing non-2xx rejection first.

### Step 4: Run worker client tests

Run:

```bash
cargo test -p hydragrow-diagnostic-worker backend_client::tests -- --nocapture
```

Expected: all backend-client tests PASS.

### Step 5: Commit the contract fix

```bash
git add hydragrow-diagnostic-worker/src/backend_client.rs
git commit -m "fix: parse health API envelope in diagnostic worker"
```

## Task 4: Verify Fleet Triggering and Read-Only Tool Boundaries

Files:

- Test/Modify: hydragrow-diagnostic-worker/src/tick.rs
- Test/Modify: hydragrow-diagnostic-worker/src/local_model.rs
- Test/Modify only if required: hydragrow-diagnostic-worker/src/diagnostic_model.rs

Interfaces:

- Consumes: HashMap<String, serde_json::Value> from DiagnosisBackend::get_fleet_hestia, existing
  evaluate_triggers, LocalLlamaDiagnosticModel, SupervisorQuery validation.
- Produces: deterministic trigger behavior and a read-only local model whose tool set cannot issue actuator commands.

### Step 1: Add explicit trigger coverage for all Hestia states

In tick.rs, extend the existing fake-fleet test with CRITICAL and RECOVERY cases and assert that only states accepted by
evaluate_triggers produce model calls. Do not reimplement trigger policy in tick.rs; the test should assert the current shared
trigger policy.

Use the existing test model and a fleet such as:

```rust
fleet.insert(
    "critical-dev".to_string(),
    serde_json::json!({"state": "CRITICAL", "reasons": ["water_level_critical"]}),
);
fleet.insert(
    "recovery-dev".to_string(),
    serde_json::json!({"state": "RECOVERY", "reasons": ["recent_intervention_recovery"]}),
);
fleet.insert(
    "comfortable-dev".to_string(),
    serde_json::json!({"state": "COMFORTABLE", "reasons": []}),
);
```

### Step 2: Run the fleet trigger tests

Run:

```bash
cargo test -p hydragrow-diagnostic-worker tick::tests -- --nocapture
```

Expected: PASS; the worker must still avoid diagnosis for non-triggering states.

### Step 3: Add a local-model tool-safety regression test

Expose or test the generated tool definitions through the existing private helper in-module. Assert that the only tool names are:

```rust
let expected = [
    "sensor_history",
    "dosing_history",
    "fsm_events",
    "health_topics",
];
```

The test must fail if any tool named like pump, dose, refill, drain, misting, mix, control, command, or actuator is present.

### Step 4: Run the local-model unit tests

Run:

```bash
cargo test -p hydragrow-diagnostic-worker local_model::tests -- --nocapture
```

Expected: PASS with no actuator-control tool exposure.

### Step 5: Commit the trigger/tool-safety coverage

```bash
git add hydragrow-diagnostic-worker/src/tick.rs hydragrow-diagnostic-worker/src/local_model.rs
git commit -m "test: lock diagnostic trigger and tool safety boundaries"
```

## Task 5: Verify Local Qwen3/llama.cpp Diagnosis Round Trips

Files:

- Modify/Test: hydragrow-diagnostic-worker/src/local_model.rs
- Test: hydragrow-diagnostic-worker/src/diagnostic_model.rs only if parser behavior needs a regression test

Interfaces:

- Consumes: llama.cpp POST /v1/chat/completions, model id qwen3-0.6b, existing DiagnosticContext,
  existing supervisor-query validation/execution.
- Produces: Diagnosis via parse_diagnosis, or the existing typed DiagnosticModelError/orchestrator fallback path.

### Step 1: Add a mocked final-answer test

Use the repository's existing HTTP mocking facility (mockito) to return:

```json
{
    "choices": [
        {
            "message": {
                "content": "{\"reason_codes\":[\"ec_out_of_range\"],\"confidence\":0.8,\"narrative\":\"EC is outside the target range.\",\"observations\":{\"current_value\":2.4,\"target_value\":1.8,\"delta\":0.6,\"window_minutes\":10,\"corroborating_evidence\":[\"recent sensor history\"]}}"
            }
        }
    ]
}
```

Assert that diagnose() returns the expected Diagnosis instead of an output-format error. The reason_codes value must be a
code accepted by SupervisorReasonCode::from_str (see hydragrow-shared/src/supervisor.rs).

### Step 2: Run the mocked final-answer test to verify the baseline

Run:

```bash
cargo test -p hydragrow-diagnostic-worker local_model::tests -- --nocapture
```

Expected: PASS once the existing local model behavior matches this response.

### Step 3: Add a mocked tool-call round-trip test

The first response must contain tool_calls in the OpenAI shape:

```json
{
    "choices": [
        {
            "message": {
                "content": null,
                "tool_calls": [
                    {
                        "id": "call-1",
                        "type": "function",
                        "function": {
                            "name": "sensor_history",
                            "arguments": "{\"device_id\":\"dev-1\",\"minutes\":10}"
                        }
                    }
                ]
            }
        }
    ]
}
```

The second response must contain a valid final diagnosis. Fake QueryBackend execution should assert the query remains device
scoped to dev-1 and then return synthetic sensor data.

### Step 4: Run the tool-round-trip test

Run:

```bash
cargo test -p hydragrow-diagnostic-worker local_model::tests -- --nocapture
```

Expected: PASS and the fake backend records exactly one scoped query.

### Step 5: Add bounded-budget tests

Cover:

```rust
assert!(matches!(
    model.diagnose(context).await,
    Err(DiagnosticModelError::BudgetExceeded(_))
));
```

for a zero/very-small wall-clock or exhausted tool-round-trip budget. Use deterministic mocked responses so the test cannot hang.

### Step 6: Run targeted worker tests

Run:

```bash
cargo test -p hydragrow-diagnostic-worker --lib -- --nocapture
```

Expected: PASS for parser, fallback, backend client, trigger, and local-model tests.

### Step 7: Commit the local model verification changes

```bash
git add hydragrow-diagnostic-worker/src/local_model.rs hydragrow-diagnostic-worker/src/diagnostic_model.rs
git commit -m "test: verify llama diagnostic round trips and budgets"
```

## Task 6: End-to-End Repository Validation and Production Verification

Files:

- Modify: none unless a test from Tasks 1–5 exposes a concrete integration defect.
- Test: workspace test suites and live API/worker verification.

Interfaces:

- Consumes: all completed backend and worker contracts.
- Produces: a verified path from production Hestia API to diagnostic worker and local model, with explicit blockers documented.

### Step 1: Run formatting

```bash
cargo fmt --all -- --check
```

Expected: exit code 0.

### Step 2: Run workspace compilation

```bash
cargo check --workspace
```

Expected: exit code 0. If an unrelated pre-existing crate fails, record the exact crate and compiler error rather than hiding it.

### Step 3: Run workspace tests

```bash
cargo test --workspace
```

Expected: all relevant tests PASS. Separate unrelated pre-existing failures from regressions in modified crates.

### Step 4: Verify the production Hestia API

After the backend image containing the route is deployed, run without printing the secret:

```bash
curl -i \
  -H "X-API-Key: $DIAGNOSTIC_API_KEY" \
  "https://hydragrow.onrender.com/api/health/hestia"
```

Expected:

- HTTP 200
- JSON body containing status: "success"
- data is an object; it may be empty when no cached device snapshot currently contains Hestia.

Then:

```bash
curl -s \
  -H "X-API-Key: $DIAGNOSTIC_API_KEY" \
  "https://hydragrow.onrender.com/api/health/hestia" | jq
```

### Step 5: Verify the diagnostic worker no longer fails at fleet-Hestia fetch

Run the worker with the existing non-secret environment variables. Confirm the old log line is absent:

```text
failed to fetch fleet hestia, skipping this tick
error=GET /api/health/hestia returned 404 Not Found
```

The next expected progression is either:

- no diagnosis because all current devices are non-triggering, or
- diagnosis orchestration starts for a WARNING/CRITICAL/watchdog device.

### Step 6: Verify local llama.cpp independently

Confirm the local server advertises the intended model id:

```bash
curl -s http://127.0.0.1:8080/v1/models | jq '.data[] | {id, owned_by}'
```

Expected: model id qwen3-0.6b after starting llama-server with --alias qwen3-0.6b.

Do not assume the runtime context size is 4096; the current running server reports n_ctx=2048, so keep worker input/tool budgets
compatible with that limit.

### Step 7: Inspect the final diff for scope creep

Run:

```bash
git diff --check
git status --short
git log --oneline -8
```

Expected:

- no whitespace errors
- only intended backend/worker/test changes
- each task represented by a focused commit

### Step 8: Document deployment constraints without guessing the Render image name

Render is using a prebuilt Docker image service. Before giving an image push command, confirm the exact registry/image/tag
configured in Render. Do not guess it. Once confirmed, the deployment sequence is:

```bash
docker build -t <CONFIRMED_IMAGE>:<CONFIRMED_TAG> .
docker push <CONFIRMED_IMAGE>:<CONFIRMED_TAG>
```

Then use Render's Deploy latest reference for the image-backed service. Do not claim that a Git push alone deploys the backend
image.

### Step 9: Commit only if final integration fixes were required

If Tasks 1–5 needed no final code changes, do not create an empty commit. If a concrete integration correction was required, use a
focused message such as:

```bash
git add <only-finally-changed-files>
git commit -m "fix: complete diagnostic hestia integration"
```

## Self-Review Checklist

- Spec coverage: fleet Hestia route, auth, cached DeviceHealthSnapshot, response envelope, worker parsing, trigger
  behavior, read-only tool boundary, llama.cpp round trips, budget handling, workspace validation, and production verification
  are covered.
- No placeholders: deployment uses <CONFIRMED_IMAGE>:<CONFIRMED_TAG> only where Render's exact configured image
  is intentionally required; no implementation TODOs are left for the engineer.
- Type consistency: extract_hestia_by_device() returns HashMap<String, HestiaAssessment>; HTTP handler
  wraps it as {status,data}; BackendClient::get_fleet_hestia() unwraps data into HashMap<String,
  serde_json::Value>, matching run_fleet_tick().
- Scope: this is one integrated vertical-slice plan because the backend endpoint and worker parser are not independently
  useful for the requested production behavior; each task still produces an independently testable deliverable.

## Completion Criteria

The feature is complete only when all of the following are true:

- GET /api/health/hestia returns HTTP 200 for a caller with health:read.
- Its data object contains only devices with valid cached HestiaAssessment values.
- BackendClient::get_fleet_hestia() correctly unwraps the {status,data} envelope.
- Fleet trigger tests show only intended Hestia/watchdog cases start diagnosis.
- Local Qwen3/llama.cpp tests cover both direct JSON diagnosis and at least one tool-call round trip.
- The diagnostic tool set remains read-only.
- cargo check --workspace and cargo test --workspace pass, or any unrelated pre-existing failure is explicitly documented.
- Production verification no longer stops on HTTP 404 for /api/health/hestia.
