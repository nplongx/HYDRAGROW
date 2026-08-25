# HYDRAGROW — Backend Module Contribution Rules

Rule chung áp dụng cho mọi subsystem: xem [`README.md`](./README.md) (index).
Phần dưới đây là rule riêng cho `hydragrow-backend`.

---

## 🗄️ Module: `db::postgres` — Core device state

**When to touch:** device_config, safety_config, dosing_calibration, sensor_calibration, pump_calibration, dosing_reports, system_events.

**Rules:**
- All functions take `pool: &PgPool` or `executor: impl Executor`. Never accept `&mut PgConnection` to avoid locking patterns.
- Use `#[instrument(skip(pool))]` on every public `async fn`.
- `get_*` functions return `Result<T>` (anyhow) and use `.context(...)` with the device_id included in the error string.
- `upsert_*` functions must use `ON CONFLICT(device_id) DO UPDATE` — never DELETE+INSERT.
- Adding a column: add a migration with `ADD COLUMN IF NOT EXISTS`, update the `FromRow` struct and all `SELECT *` queries to explicit column lists.
- Changing a CHECK constraint: always DROP then ADD (Postgres cannot modify constraints in place). Pair with an `_undo_` migration.

**Test checklist for new functions:**
- [ ] Happy path roundtrip (insert then read back)
- [ ] Not-found returns `Err` / `None` as documented
- [ ] Upsert does not duplicate on re-insert

---

## 🌱 Module: `db::recipes` — Crop recipe templates

**When to touch:** crop_recipes, crop_recipe_stages, device_active_recipes.

**Rules:**
- `crop_recipe_stages.stage_order` is the sort key; always `ORDER BY stage_order` in list queries.
- DELETE on `crop_recipes` must cascade — do not issue separate DELETE on stages.
- `device_active_recipes` has a `UNIQUE (device_id)` constraint. Use `ON CONFLICT (device_id) DO UPDATE` for activation.
- Never store derived values (e.g. total_duration_days) — compute them in Rust.
- `target_config JSONB` (from the Aug 19 design, superseded by flat columns in Aug 20) must not be re-introduced. All targets are explicit columns.

**Test checklist:**
- [ ] insert + list
- [ ] delete cascades stages
- [ ] stage list is ordered by stage_order
- [ ] activating a recipe on an already-active device updates rather than errors

---

## 👤 Module: `db::users` — Firebase UID → internal account

**When to touch:** the `users` table.

**Rules:**
- Never store a Firebase JWT or raw password in the users table.
- `scopes` is a `Vec<String>` / `TEXT[]` column. Scope strings follow the pattern `resource:action` (e.g. `device:read`).
- `is_active = FALSE` is a soft ban; the row must never be deleted.
- `find_active_by_firebase_uid` is called on every authenticated request — keep it on the primary key index (`firebase_uid` has a UNIQUE constraint). Do not add JOINs to this function.
- Adding scopes: always provide the full scope list — the upsert overwrites the entire array. Do not `array_append` in SQL.

**Test checklist:**
- [ ] upsert creates record
- [ ] upsert on conflict updates email/display_name but preserves is_active
- [ ] find returns None for unknown uid
- [ ] find returns None for is_active = FALSE user

---

## 🔌 Module: `db::device_ownership` — Multi-tenant device claim

**When to touch:** the `device_ownership` table.

**Rules:**
- MQTT credentials are generated **once** at first claim and never regenerated automatically. If a user needs new credentials they must unclaim and re-claim.
- Never return the `mqtt_password_hash` column to a client — only return `ClaimedMqttCredentials` (plaintext password) immediately after first claim.
- `claim_device` must check for an existing `mqtt_username` in the same transaction logic before generating credentials (see the current implementation pattern).
- `unclaim_device` returns `rows_affected` — callers must check for 0 (not an error, but indicates the claim did not exist).
- `is_owner_of_all` with an empty slice must return `true` (vacuous truth) — do not change this.

**Test checklist:**
- [ ] First claim returns credentials
- [ ] Second claim does not regenerate credentials
- [ ] unclaim removes ownership
- [ ] is_owner returns false for different user
- [ ] is_owner_of_all returns false when one device not owned

---

## 🌿 Module: `api::recipe` — Recipe HTTP endpoints

**When to touch:** `src/api/recipe.rs`, recipe-related routes in `src/api/mod.rs`.

**Rules:**
- All DB access goes through `crate::db::recipes::*` — no raw `sqlx::query` in this file.
- Recipe apply (`PUT /devices/{device_id}/recipe`) must publish an MQTT command and update `device_active_recipes` **atomically via a transaction** — if the MQTT publish fails, roll back the DB write.
- Recipe clear must also publish MQTT and remove `device_active_recipes` for the device.
- Request validation happens in the handler before any DB call. Return `400 Bad Request` with a JSON `{ "error": "..." }` body for invalid input.
- `ApplyRecipeRequest` may contain either `recipe_id` (use existing template) or `recipe` (inline, create a temporary recipe). Both branches must be tested.

**Test checklist:**
- [ ] GET /recipes returns empty list for fresh DB
- [ ] POST /recipes creates and GET retrieves
- [ ] DELETE /recipes/:id removes recipe and stages
- [ ] PUT /devices/:id/recipe with unknown recipe_id returns 404
- [ ] GET /devices/:id/recipe returns active recipe after apply

---

## 📡 Module: `mqtt::handlers` — Device message ingestion

**When to touch:** `src/mqtt/handlers/*.rs`.

**Rules:**
- Handlers are **fire-and-forget** — they must not block the MQTT loop. Spawn a `tokio::spawn` if DB writes may be slow.
- Every handler that writes to DB must write a `system_events` row with level `"info"` or `"warning"` at minimum.
- Do not call `unwrap()` on MQTT payload deserialization — log the error and return early.
- Topic pattern changes must be coordinated with `hydragrow-shared::topics` — never hardcode topic strings in handler code.
- Adding a new handler: register it in `src/mqtt/mod.rs` dispatch table; add a unit test that feeds a raw JSON payload string and asserts the expected DB side-effect.

**Test checklist:**
- [ ] Valid payload writes expected DB row
- [ ] Malformed payload logs error but does not panic
- [ ] Unknown device_id does not create spurious DB records

---

## 🔐 Module: `api::middleware::auth` — JWT / Firebase auth

**When to touch:** `src/api/middleware/auth.rs`.

**Rules:**
- The middleware must **never** cache user records beyond the lifetime of a single request. User suspension (`is_active = FALSE`) must take effect on the next request.
- Scope checks use the `scopes` field from `UserRecord`. Adding a new protected scope requires: (1) updating the scope string constant in `src/api/scope_definitions.rs`, (2) adding it to the relevant endpoint extractor, (3) updating `docs/superpowers/specs/module-rules.md` (this file).
- Auth errors always return `401 Unauthorized` with `WWW-Authenticate: Bearer` header — never `403` for missing token.

**Test checklist:**
- [ ] Valid token with correct scope → 200
- [ ] Valid token with missing scope → 403
- [ ] Expired token → 401
- [ ] is_active = FALSE user → 401

---

## 🔧 Migrations Checklist (run before every PR)

```bash
# From hydragrow-backend/
sqlx migrate run           # apply pending migrations
cargo test -- --test-threads=1  # all tests must pass
```

PRs that change schema without a migration, or add DB functions without tests, are blocked.
