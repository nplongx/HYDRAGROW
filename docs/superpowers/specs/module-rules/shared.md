# HYDRAGROW — `hydragrow-shared` Module Rules

`hydragrow-shared` là **nguồn sự thật duy nhất** cho type và hằng số dùng chung giữa backend, controller-core, và (gián tiếp qua JSON) firmware + frontend. Không subsystem nào được định nghĩa lại type hoặc topic MQTT song song với crate này.

**Khi nào chạm module này:** thêm/sửa field trong `SensorData`, `PumpStatus`, `FsmSnapshot`, `CropRecipe`, `UnifiedSystemLog`; thêm/sửa hằng số trong `MqttTopics`; đổi `schema_version`.

## Rules

- File nào giữ 1 nhóm type (`fsm.rs`, `recipe.rs`, `events.rs`, `log.rs`, `topics.rs`, `hestia.rs`) — thêm type mới vào đúng file theo domain, không tạo file `misc.rs`/`common.rs`.
- Mọi struct dùng để serialize qua MQTT hoặc lưu DB dạng JSON/JSONB phải có `#[derive(Debug, Clone, Serialize, Deserialize)]` tối thiểu, và một field `schema_version: u32` nếu struct đó có thể tiến hoá theo thời gian (đã áp dụng cho `CropRecipe` — theo mẫu đó).
- `MqttTopics` (`topics.rs`) là nơi DUY NHẤT được phép build chuỗi topic `AGITECH/{device_id}/...`. Không hardcode chuỗi topic ở backend, firmware, hay frontend — luôn gọi hàm từ `MqttTopics` (Rust) hoặc copy nguyên format string sang tài liệu/const tương ứng phía C++ (`ESP32-C3-SENSOR-NODE` không link được crate Rust, nên phải giữ đồng bộ thủ công — xem [firmware-sensor.md](./firmware-sensor.md)).

### Thay đổi tương thích ngược

- Thêm field mới vào struct đã có: field mới PHẢI có `#[serde(default)]` hoặc kiểu `Option<T>` — thiết bị firmware cũ vẫn gửi payload không có field đó và không được phép làm backend deserialize fail.
- Đổi tên field: KHÔNG đổi trực tiếp. Thêm field mới, giữ field cũ với `#[serde(alias = "ten_cu")]` tối thiểu 1 minor version, xóa field cũ ở PR riêng sau khi mọi firmware đã update (theo dõi ở [firmware-controller.md](./firmware-controller.md)).
- Đổi kiểu enum có variant mới (VD: thêm `FaultCode` mới): phải thêm test roundtrip cho variant đó trong `tests/schema_roundtrip_and_snapshots.rs` trong CÙNG PR, không để PR sau bổ sung.
- Snapshot test (`insta`) fail sau khi đổi có chủ đích: chạy `INSTA_UPDATE=always cargo test --test schema_roundtrip_and_snapshots`, review diff snapshot bằng mắt trước khi commit — không update snapshot mà không đọc diff.

## Test checklist cho mọi type mới/sửa

- [ ] Roundtrip serialize → deserialize giữ nguyên giá trị (test trong `tests/schema_roundtrip_and_snapshots.rs`)
- [ ] Nếu có field optional mới: test deserialize một payload JSON **thiếu** field đó vẫn thành công (mô phỏng firmware cũ)
- [ ] Nếu thêm topic mới vào `MqttTopics`: test format string đúng pattern `AGITECH/{device_id}/<suffix>`
