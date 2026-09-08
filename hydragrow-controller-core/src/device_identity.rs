//! Fleet device-identity helpers (host-testable).
//!
//! Release firmware contains no per-device identity: at boot the identity
//! resolves as NVS `device_id` → compile-time default (dev builds only) →
//! deterministic factory identity derived from the chip MAC → and a later
//! claim/provision command may persist the logical device id in NVS.

/// Format a deterministic factory identity from a 6-byte station MAC.
pub fn format_factory_id(mac: &[u8; 6]) -> String {
    format!(
        "esp32c3-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        mac[0], mac[1], mac[2], mac[3], mac[4], mac[5]
    )
}

/// Resolve the runtime device id from NVS, compile default, and factory id.
/// Empty NVS values are treated as missing so a blank key can never become
/// the fleet-wide identity.
pub fn resolve_device_id(nvs_id: Option<&str>, compile_default: &str, factory_id: &str) -> String {
    if let Some(id) = nvs_id.map(str::trim).filter(|id| !id.is_empty()) {
        return id.to_string();
    }
    if !compile_default.trim().is_empty() {
        return compile_default.trim().to_string();
    }
    factory_id.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_device_id_derives_stable_factory_identity() {
        let id = resolve_device_id(None, "", "esp32c3-a1b2c3d4e5f6");
        assert_eq!(id, "esp32c3-a1b2c3d4e5f6");
        // Stable across calls.
        assert_eq!(
            resolve_device_id(None, "", "esp32c3-a1b2c3d4e5f6"),
            "esp32c3-a1b2c3d4e5f6"
        );
    }

    #[test]
    fn blank_nvs_id_falls_through_to_factory_identity() {
        assert_eq!(
            resolve_device_id(Some("   "), "", "esp32c3-a1b2c3d4e5f6"),
            "esp32c3-a1b2c3d4e5f6"
        );
    }

    #[test]
    fn provisioned_device_id_overrides_factory_identity() {
        assert_eq!(
            resolve_device_id(Some("greenhouse-01"), "", "esp32c3-a1b2c3d4e5f6"),
            "greenhouse-01"
        );
        assert_eq!(
            resolve_device_id(Some("greenhouse-01"), "device_001", "esp32c3-a1b2c3d4e5f6"),
            "greenhouse-01"
        );
    }

    #[test]
    fn compile_default_used_only_when_nvs_missing() {
        assert_eq!(
            resolve_device_id(None, "device_001", "esp32c3-a1b2c3d4e5f6"),
            "device_001"
        );
    }

    #[test]
    fn factory_id_format_is_deterministic() {
        assert_eq!(
            format_factory_id(&[0xa1, 0xb2, 0xc3, 0xd4, 0xe5, 0xf6]),
            "esp32c3-a1b2c3d4e5f6"
        );
    }
}
