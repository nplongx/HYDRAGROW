use hydragrow_shared::{CURRENT_SCHEMA_VERSION, MIN_SUPPORTED_SCHEMA_VERSION};
use tracing::warn;

pub fn validate_payload_schema(
    payload_type: &str,
    device_id: &str,
    schema_version: Option<u16>,
) -> bool {
    let version = schema_version.unwrap_or(1);

    if version < MIN_SUPPORTED_SCHEMA_VERSION || version > CURRENT_SCHEMA_VERSION {
        warn!(
            payload_type,
            device_id,
            version,
            min_supported = MIN_SUPPORTED_SCHEMA_VERSION,
            current = CURRENT_SCHEMA_VERSION,
            "Từ chối payload do schema_version không được hỗ trợ"
        );
        return false;
    }

    if version < CURRENT_SCHEMA_VERSION {
        warn!(
            payload_type,
            device_id,
            version,
            current = CURRENT_SCHEMA_VERSION,
            "Nhận payload schema cũ (đang chạy chế độ tương thích N-1)"
        );
    }

    true
}
