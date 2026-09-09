use serde::Deserialize;
use std::collections::HashMap;
use std::sync::LazyLock;

/// Bản sao CÓ CHỦ ĐÍCH của hydragrow-frontend/src/lib/automation/device-config-keys.json.
/// KHÔNG include_str! trực tiếp vào thư mục hydragrow-frontend ở đây: Docker
/// build của backend (./Dockerfile, hydragrow-backend/Dockerfile) chỉ COPY
/// hydragrow-shared + hydragrow-backend vào build context — hydragrow-frontend
/// không tồn tại ở đó, nên include_str! xuyên thư mục sẽ làm build release
/// lỗi compile (CI không phát hiện được vì backend-ci.yml checkout toàn bộ
/// repo). Test `sync_with_frontend` bên dưới đối chiếu 2 file để tránh lệch —
/// sửa danh sách key thì phải sửa CẢ HAI file giống hệt nhau.
const REGISTRY_JSON: &str = include_str!("../../config/device-config-keys.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueType {
    Float,
    Integer,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ConfigKeyDef {
    pub key: String,
    #[serde(rename = "valueType")]
    pub value_type: ValueType,
    pub min: f64,
    pub max: f64,
    pub unit: String,
    pub step: f64,
    pub label: String,
    #[serde(rename = "sourceGroup")]
    pub source_group: String,
    #[serde(rename = "defaultVal")]
    pub default_val: f64,
}

static REGISTRY: LazyLock<HashMap<String, ConfigKeyDef>> = LazyLock::new(|| {
    let defs: Vec<ConfigKeyDef> = serde_json::from_str(REGISTRY_JSON)
        .expect("device-config-keys.json phải là JSON hợp lệ — lỗi build, không phải lỗi runtime");
    defs.into_iter().map(|d| (d.key.clone(), d)).collect()
});

/// Tra 1 key. `None` nếu key không nằm trong registry (không được phép
/// Đọc/Ghi đè qua Flow).
pub fn lookup(key: &str) -> Option<&'static ConfigKeyDef> {
    REGISTRY.get(key)
}

/// Toàn bộ key hợp lệ — dùng để dựng dropdown / validate.
pub fn all_keys() -> Vec<&'static str> {
    REGISTRY.keys().map(|s| s.as_str()).collect()
}

/// Kẹp `val` theo [min, max] đã khai báo cho `key`. Key không có trong
/// registry -> không kẹp (caller quyết định coi đó là lỗi "unknown key" ở
/// tầng khác, xem config_override::write_field).
pub fn clamp(key: &str, val: f64) -> (f64, bool) {
    let Some(def) = lookup(key) else {
        return (val, false);
    };
    if val < def.min {
        (def.min, true)
    } else if val > def.max {
        (def.max, true)
    } else {
        (val, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup_finds_a_known_key() {
        let def = lookup("ec_target").expect("ec_target phải có trong registry");
        assert_eq!(def.min, 0.8);
        assert_eq!(def.max, 3.2);
        assert_eq!(def.value_type, ValueType::Float);
    }

    #[test]
    fn lookup_returns_none_for_an_unknown_key() {
        assert!(lookup("dose_max_ml").is_none());
        assert!(lookup("not_a_real_key").is_none());
    }

    #[test]
    fn clamp_leaves_an_in_range_value_untouched() {
        assert_eq!(clamp("ec_target", 1.8), (1.8, false));
    }

    #[test]
    fn clamp_pulls_a_too_low_value_up_to_min() {
        assert_eq!(clamp("ec_target", 0.1), (0.8, true));
    }

    #[test]
    fn clamp_pulls_a_too_high_value_down_to_max() {
        assert_eq!(clamp("ec_target", 99.0), (3.2, true));
    }

    #[test]
    fn clamp_is_a_no_op_for_a_key_outside_the_registry() {
        assert_eq!(clamp("control_mode", 42.0), (42.0, false));
    }

    /// Chỉ đọc thư mục frontend trong `cargo test` (CI checkout đầy đủ repo,
    /// xem .github/workflows/backend-ci.yml) — KHÔNG dùng ngoài #[cfg(test)],
    /// nếu không build release trong Docker sẽ lỗi vì hydragrow-frontend
    /// không có trong build context của backend.
    #[test]
    fn backend_copy_matches_frontend_copy_byte_for_byte() {
        const FRONTEND_COPY: &str =
            include_str!("../../../hydragrow-frontend/src/lib/automation/device-config-keys.json");
        assert_eq!(
            REGISTRY_JSON, FRONTEND_COPY,
            "hydragrow-backend/config/device-config-keys.json đã lệch với \
            hydragrow-frontend/src/lib/automation/device-config-keys.json — \
            đây là 2 bản sao có chủ đích (không phải 1 file share qua include_str! \
            xuyên thư mục), sửa danh sách config key thì phải sửa CẢ HAI file \
            giống hệt nhau."
        );
    }

    #[test]
    fn all_keys_matches_the_json_file_exactly() {
        let mut keys = all_keys();
        keys.sort();
        assert_eq!(
            keys,
            vec![
                "delay_between_a_and_b_sec",
                "ec_target",
                "ec_tolerance",
                "ph_target",
                "ph_tolerance",
            ]
        );
    }
}
