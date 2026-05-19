# hydragrow-shared

Thư viện định nghĩa schema dùng chung giữa firmware, backend và frontend.

## Schema versioning

Các payload chính đã có `schema_version` dạng `Option<u16>`:
- `UnifiedSystemLog` (system log)
- `SensorData` (sensor update)
- `MqttCommandPayload` / command payload

### Quy ước tương thích

- **Schema hiện tại**: `2`
- **Backend hỗ trợ tối thiểu**: `N-1` (tức là `1` khi current là `2`).
- Field mới phải để `Option` + `#[serde(default)]` để deserialize an toàn với payload cũ.
- Với payload không có `schema_version`, backend mặc định hiểu là version `1` để tương thích ngược.

### Firmware ↔ Backend compatibility matrix

| Firmware payload `schema_version` | Backend (current schema = 2) | Trạng thái |
|---|---|---|
| 2 | Accepted | ✅ Native path |
| 1 | Accepted | ⚠️ Compatibility mode (N-1), backend log warning |
| 0 hoặc < 1 | Rejected | ❌ Unsupported (too old) |
| > 2 | Rejected | ❌ Unsupported (too new) |
| Missing (`None`) | Accepted as `1` | ⚠️ Legacy compatibility path |

> Khi bump schema mới, cập nhật `CURRENT_SCHEMA_VERSION` và giữ hỗ trợ tối thiểu `N-1`.
