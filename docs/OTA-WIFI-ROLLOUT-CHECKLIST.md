# OTA + WiFi Provisioning — Staged Rollout Checklist (v0.9.0)

Required order. Do not skip stages; each stage gates the next.

## Stage 0 — Preconditions (all must be green)

- [ ] `cargo test` green: hydragrow-shared, hydragrow-controller-core, hydragrow-backend (`--test-threads=4`), frontend `npm test -- --run` + `npm run build`.
- [ ] firmware-controller-ci green on the release branch (cargo check, fmt, clippy, ESP32-C3 target build).
- [ ] Acceptance/evidence contracts validate:
  `validate_acceptance_contract.py docs/acceptance/OTA-WIFI-TRANSACTION-001.json` and
  `validate_evidence_contract.py docs/acceptance/OTA-WIFI-TRANSACTION-001.json docs/evidence/OTA-WIFI-TRANSACTION-001.json`.
- [ ] Migrations `20260907000000_wifi_config_delivery.sql` and `20260907000001_device_hardware_binding.sql` applied to staging and production databases.

## Stage 1 — Deploy backend first

- [ ] Deploy backend with the combined `/ota/trigger`, `GET /wifi`, and `wifi_config_status` routing.
- [ ] Verify `GET /devices/{id}/wifi` returns SSID metadata only (no `password` key anywhere in the response).
- [ ] Verify legacy `POST /devices/{id}/wifi` still works (migration path).

## Stage 2 — Deploy frontend second

- [ ] Deploy frontend with combined OTA+WiFi actions.
- [ ] Verify known SSIDs render with blank masked passwords and the `WiFi config vN — <state>` line appears.

## Stage 3 — Provision/upgrade one controller manually

- [ ] Flash v0.9.0 on one bench controller (no WiFi secrets in the image; factory id derived from MAC).
- [ ] Confirm NVS `device_id` resolves (factory `esp32c3-<mac>`) and the device checks in under its topic.

## Stage 4 — Verify NVS active/pending behavior

- [ ] Submit a new WiFi list via the combined endpoint; confirm NVS holds pending + `Prepared` while active is untouched.

## Stage 5 — OTA one pilot device

- [ ] Old WiFi works before OTA; new WiFi submitted from UI; DB contains only SSID metadata.
- [ ] OTA completes; reboot occurs; new WiFi connects; new firmware reports healthy; delivery state becomes `applied`.

## Stage 6 — Verify rollback test

- [ ] Submit an intentionally wrong password; verify OTA completes, boot fails on pending, device rolls back to old WiFi, delivery state becomes `rolled_back`.
- [ ] Submit a stale `config_version`; verify HTTP 409 and no device change.

## Stage 7 — Verify DB contains SSID only

- [ ] `SELECT to_jsonb(t) FROM device_wifi_config t;` shows no `password` key on any row.

## Stage 8 — Roll out to 5 devices, then 20, then full fleet

- [ ] 5 devices: all `applied`, no rollbacks unexplained.
- [ ] 20 devices: same gate.
- [ ] Full fleet: monitor `wifi_config_status` delivery states; investigate any `rolled_back`/`rejected` before proceeding.

## Release cut (v0.9.0)

- [ ] `ESP32-C3-CONTROLLER-NODE/Cargo.toml` is `0.9.0` (done in this branch).
- [ ] Tag/version equality is enforced by `firmware-controller-release.yml` (keep as-is).
- [ ] `git push origin main && git tag v0.9.0 && git push origin v0.9.0` (operator action — NOT done by automation).
- [ ] Release contains `firmware.bin` (exact name, OTA-compatible) plus `firmware.bin.sha256`.
- [ ] Verify: `firmware.bin` size <= OTA slot size; SHA-256 asset matches; release version equals firmware `CARGO_PKG_VERSION`.

## Notes

- ESP32 target build and hardware pilot steps cannot run in this environment (no ESP toolchain / no devices); they are operator/CI responsibilities.
- Legacy `POST /wifi` and `update_wifi_list` stay until Task 14 retires them after fleet migration.
