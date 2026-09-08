//! Host-testable OTA release verification helpers.
//!
//! The ESP32 firmware (`hw/ota.rs`) calls these exact functions; the logic is
//! kept here so `cargo test` on the host covers version policy, release
//! parsing, and SHA-256 verification without ESP-IDF hardware.

use sha2::{Digest, Sha256};

/// Release metadata extracted from the GitHub Releases API response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FirmwareReleaseMeta {
    pub tag: String,
    pub firmware_url: String,
    /// Download URL of the `firmware.bin.sha256` asset, if published.
    pub sha256_url: Option<String>,
}

/// Extract tag + `firmware.bin` (+ optional `firmware.bin.sha256`) asset URLs.
pub fn parse_release_metadata(json: &serde_json::Value) -> Result<FirmwareReleaseMeta, String> {
    let tag = json["tag_name"].as_str().unwrap_or("").to_string();
    if tag.is_empty() {
        return Err("tag_name missing in GitHub response".into());
    }
    let assets = json["assets"]
        .as_array()
        .ok_or_else(|| "assets missing in GitHub response".to_string())?;
    let asset_url = |name: &str| {
        assets
            .iter()
            .find(|a| a["name"].as_str() == Some(name))
            .and_then(|a| a["browser_download_url"].as_str())
            .map(|s| s.to_string())
    };
    let firmware_url =
        asset_url("firmware.bin").ok_or_else(|| "firmware.bin asset not found".to_string())?;
    Ok(FirmwareReleaseMeta {
        tag,
        firmware_url,
        sha256_url: asset_url("firmware.bin.sha256"),
    })
}

/// Parse a `vMAJOR.MINOR.PATCH` tag into numeric parts. Rejects malformed tags.
pub fn parse_semver(tag: &str) -> Result<(u64, u64, u64), String> {
    let stripped = tag.strip_prefix('v').unwrap_or(tag);
    let mut parts = stripped.split('.');
    let parse_part = |part: Option<&str>| {
        part.filter(|p| !p.is_empty())
            .and_then(|p| p.parse::<u64>().ok())
            .ok_or_else(|| format!("malformed version tag: {tag}"))
    };
    let major = parse_part(parts.next())?;
    let minor = parse_part(parts.next())?;
    let patch = parse_part(parts.next())?;
    if parts.next().is_some() {
        return Err(format!("malformed version tag: {tag}"));
    }
    Ok((major, minor, patch))
}

/// Returns `Ok(true)` only when `candidate` is strictly newer than `current`.
/// Same version, downgrades, and malformed tags are all rejected, so a
/// compromised or stale release can never replace running firmware unless a
/// deliberate administrative override path is used.
pub fn is_upgrade(current: &str, candidate: &str) -> Result<bool, String> {
    let current_v = parse_semver(current)?;
    let candidate_v = parse_semver(candidate)?;
    if candidate_v == current_v {
        return Err(format!("already on version {current}"));
    }
    if candidate_v < current_v {
        return Err(format!("refusing downgrade from {current} to {candidate}"));
    }
    Ok(true)
}

/// Verify streamed firmware bytes against the expected hex digest.
/// A missing digest is a hard failure: never commit an unverified image.
pub fn verify_firmware_image(data: &[u8], expected_hex: Option<&str>) -> Result<(), String> {
    let expected = expected_hex
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "missing SHA-256 digest for firmware image".to_string())?;
    let mut hasher = Sha256::new();
    hasher.update(data);
    let actual = hex::encode(hasher.finalize());
    if actual.eq_ignore_ascii_case(expected) {
        Ok(())
    } else {
        Err("firmware SHA-256 mismatch".into())
    }
}

/// Parse the body of a `firmware.bin.sha256` checksum file
/// (`<hex>[  filename]` per `sha256sum` output) into the bare hex digest.
pub fn parse_checksum_file(body: &str) -> Option<String> {
    body.split_whitespace().next().map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_same_version() {
        assert!(is_upgrade("v0.9.0", "v0.9.0").is_err());
    }

    #[test]
    fn rejects_downgrade() {
        assert!(is_upgrade("v0.9.0", "v0.8.0").is_err());
        assert!(is_upgrade("v0.9.0", "v0.9.0").is_err());
    }

    #[test]
    fn rejects_malformed_version() {
        assert!(is_upgrade("v0.9.0", "latest").is_err());
        assert!(is_upgrade("v0.9.0", "v1.2").is_err());
        assert!(is_upgrade("v0.9.0", "").is_err());
        assert!(is_upgrade("not-a-version", "v1.0.0").is_err());
    }

    #[test]
    fn accepts_newer_version() {
        assert_eq!(is_upgrade("v0.8.0", "v0.9.0"), Ok(true));
        assert_eq!(is_upgrade("v0.9.0", "v1.0.0"), Ok(true));
    }

    #[test]
    fn rejects_missing_sha256() {
        assert!(verify_firmware_image(b"bytes", None).is_err());
        assert!(verify_firmware_image(b"bytes", Some("  ")).is_err());
    }

    #[test]
    fn accepts_matching_sha256() {
        let data = b"firmware-bytes";
        let mut hasher = Sha256::new();
        hasher.update(data);
        let digest = hex::encode(hasher.finalize());
        assert!(verify_firmware_image(data, Some(&digest)).is_ok());
        assert!(verify_firmware_image(data, Some(&digest.to_uppercase())).is_ok());
        assert!(verify_firmware_image(b"other-bytes", Some(&digest)).is_err());
    }

    #[test]
    fn parses_release_metadata_with_checksum_asset() {
        let json = serde_json::json!({
            "tag_name": "v0.9.0",
            "assets": [
                {"name": "firmware.bin", "browser_download_url": "https://example.com/firmware.bin"},
                {"name": "firmware.bin.sha256", "browser_download_url": "https://example.com/firmware.bin.sha256"}
            ]
        });
        let meta = parse_release_metadata(&json).unwrap();
        assert_eq!(meta.tag, "v0.9.0");
        assert!(meta.firmware_url.ends_with("firmware.bin"));
        assert!(meta.sha256_url.unwrap().ends_with("firmware.bin.sha256"));
    }

    #[test]
    fn release_without_firmware_asset_is_rejected() {
        let json = serde_json::json!({"tag_name": "v0.9.0", "assets": []});
        assert!(parse_release_metadata(&json).is_err());
    }

    #[test]
    fn parses_sha256sum_file_body() {
        assert_eq!(
            parse_checksum_file("abc123  firmware.bin\n"),
            Some("abc123".to_string())
        );
        assert_eq!(parse_checksum_file("abc123"), Some("abc123".to_string()));
        assert_eq!(parse_checksum_file(""), None);
    }
}
