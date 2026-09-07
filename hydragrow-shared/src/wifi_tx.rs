//! Pure host-testable state model for transactional WiFi provisioning.
//!
//! The ESP32 NVS layer (`wifi_store.rs`) persists these transitions; this
//! module defines the semantics so they can be unit-tested on the host.

use serde::{Deserialize, Serialize};

use crate::{WifiCandidate, WifiCredentialList, WifiProvisionConfig, WifiSecretAction};

/// A versioned WiFi credential set (active or pending).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionedWifiConfig {
    pub credentials: WifiCredentialList,
    pub version: i64,
}

/// Transaction marker persisted alongside the NVS keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WifiTxState {
    /// No pending transaction (steady state).
    Committed,
    /// Pending config staged in NVS, awaiting OTA commit + boot apply.
    Prepared,
}

impl WifiTxState {
    pub fn from_state(s: &str) -> Self {
        match s {
            "Prepared" | "CommitPending" => Self::Prepared,
            _ => Self::Committed,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Committed => "Committed",
            Self::Prepared => "Prepared",
        }
    }
}

/// Pure active/pending WiFi state machine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WifiConfigState {
    pub active: WifiCredentialList,
    pub active_version: i64,
    pub pending: Option<VersionedWifiConfig>,
    pub tx_state: WifiTxState,
}

impl WifiConfigState {
    pub fn new(active: WifiCredentialList) -> Self {
        Self {
            active,
            active_version: 0,
            pending: None,
            tx_state: WifiTxState::Committed,
        }
    }

    pub fn prepare_pending(mut self, credentials: WifiCredentialList, version: i64) -> Self {
        self.pending = Some(VersionedWifiConfig {
            credentials,
            version,
        });
        self.tx_state = WifiTxState::Prepared;
        self
    }

    pub fn commit_pending(mut self) -> Self {
        if let Some(pending) = self.pending.take() {
            self.active = pending.credentials;
            self.active_version = pending.version;
        }
        self.tx_state = WifiTxState::Committed;
        self
    }

    pub fn rollback_pending(mut self) -> Self {
        self.pending = None;
        self.tx_state = WifiTxState::Committed;
        self
    }

    /// New config versions must strictly increase for Set/Replace transactions.
    pub fn is_fresh_version(&self, version: i64) -> bool {
        version > self.active_version
    }
}

/// Boot-time decision derived from the persisted transaction marker.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BootWifiDecision {
    /// Try pending credentials first, fall back to active on failure.
    TryPendingThenActive,
    /// No pending transaction; use active list only.
    ActiveOnly,
}

impl BootWifiDecision {
    pub fn from_state(state: &str) -> Self {
        match WifiTxState::from_state(state) {
            WifiTxState::Prepared => Self::TryPendingThenActive,
            WifiTxState::Committed => Self::ActiveOnly,
        }
    }
}

/// Device → backend provisioning result. Never carries passwords.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WifiConfigStatus {
    #[serde(rename = "type")]
    pub event_type: String,
    pub device_id: String,
    pub config_version: i64,
    /// One of: applied | rolled_back | rejected.
    pub state: String,
    pub ssid_count: usize,
}

impl WifiConfigStatus {
    pub fn new(device_id: &str, config_version: i64, state: &str, ssid_count: usize) -> Self {
        Self {
            event_type: "wifi_config_status".into(),
            device_id: device_id.into(),
            config_version,
            state: state.into(),
            ssid_count,
        }
    }
}

/// Validate an OTA+WiFi provision transaction at the command boundary.
/// Never logs or retains passwords; only counts and metadata are inspected.
pub fn validate_provision_config(
    config: &WifiProvisionConfig,
    current_version: i64,
) -> Result<(), String> {
    validate_provision_structure(config)?;
    if config.config_version <= current_version {
        return Err(format!(
            "stale wifi config version: {} <= current {current_version}",
            config.config_version
        ));
    }
    Ok(())
}

/// Structural validation without the NVS version check, for use at the
/// command boundary where the active version is not yet available.
/// The dispatcher re-runs the full check (including freshness) with NVS.
pub fn validate_provision_structure(config: &WifiProvisionConfig) -> Result<(), String> {
    const MAX_ENTRIES: usize = 8;
    if config.entries.is_empty() {
        return Err("wifi config must contain at least one entry".into());
    }
    if config.entries.len() > MAX_ENTRIES {
        return Err(format!(
            "too many wifi entries: {} > {MAX_ENTRIES}",
            config.entries.len()
        ));
    }
    let mut seen_priorities = std::collections::HashSet::new();
    for entry in &config.entries {
        let ssid_len = entry.ssid.trim().len();
        if ssid_len == 0 || ssid_len > 32 {
            return Err("ssid must be 1..=32 bytes after trimming".into());
        }
        if let Some(password) = &entry.password {
            if password.len() > 64 {
                return Err("password must be 0..=64 bytes".into());
            }
        }
        entry.validate()?;
        if !seen_priorities.insert(entry.priority) {
            return Err(format!("duplicate wifi priority: {}", entry.priority));
        }
    }
    Ok(())
}

/// Resolve a validated provision config into concrete credentials.
/// `Keep` entries reuse the password from the active list; `Clear` entries
/// are dropped. Fails when a `Keep` SSID is unknown or nothing remains.
pub fn resolve_provision_credentials(
    config: &WifiProvisionConfig,
    active: &WifiCredentialList,
) -> Result<WifiCredentialList, String> {
    let mut candidates = Vec::with_capacity(config.entries.len());
    for entry in &config.entries {
        match entry.secret_action {
            WifiSecretAction::Set => {
                let password = entry.password.clone().unwrap_or_default();
                candidates.push(WifiCandidate {
                    ssid: entry.ssid.trim().to_string(),
                    password,
                    priority: entry.priority,
                });
            }
            WifiSecretAction::Keep => {
                let known = active
                    .candidates
                    .iter()
                    .find(|c| c.ssid.trim() == entry.ssid.trim());
                match known {
                    Some(candidate) => candidates.push(WifiCandidate {
                        ssid: entry.ssid.trim().to_string(),
                        password: candidate.password.clone(),
                        priority: entry.priority,
                    }),
                    None => {
                        return Err("keep requested for unknown ssid (metadata only)".into());
                    }
                }
            }
            WifiSecretAction::Clear => {}
        }
    }
    if candidates.is_empty() {
        return Err("provision resolves to an empty credential list".into());
    }
    candidates.sort_by_key(|c| c.priority);
    Ok(WifiCredentialList { candidates })
}

pub fn active(ssid: &str, password: &str) -> WifiCredentialList {
    WifiCredentialList {
        candidates: vec![WifiCandidate {
            ssid: ssid.into(),
            password: password.into(),
            priority: 0,
        }],
    }
}

pub fn set(ssid: &str, password: &str) -> WifiCredentialList {
    active(ssid, password)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pending_wifi_does_not_replace_active_wifi_before_commit() {
        let state = WifiConfigState::new(active("old", "old-pass"))
            .prepare_pending(set("new", "new-pass"), 2);
        assert_eq!(state.active.candidates[0].ssid, "old");
        assert_eq!(state.pending.unwrap().credentials.candidates[0].ssid, "new");
    }

    #[test]
    fn commit_pending_promotes_new_credentials() {
        let state = WifiConfigState::new(active("old", "old-pass"))
            .prepare_pending(set("new", "new-pass"), 2);
        let committed = state.commit_pending();
        assert_eq!(committed.active.candidates[0].ssid, "new");
        assert!(committed.pending.is_none());
    }

    #[test]
    fn failed_pending_boot_keeps_old_credentials() {
        let state = WifiConfigState::new(active("old", "old-pass"))
            .prepare_pending(set("new", "new-pass"), 2);
        let rolled_back = state.rollback_pending();
        assert_eq!(rolled_back.active.candidates[0].ssid, "old");
        assert!(rolled_back.pending.is_none());
    }

    #[test]
    fn boot_prefers_pending_only_when_transaction_requires_apply() {
        let d = BootWifiDecision::from_state("Prepared");
        assert_eq!(d, BootWifiDecision::TryPendingThenActive);
    }

    #[test]
    fn boot_uses_active_when_no_pending_transaction_exists() {
        let d = BootWifiDecision::from_state("Committed");
        assert_eq!(d, BootWifiDecision::ActiveOnly);
    }

    #[test]
    fn stale_version_is_not_fresh() {
        let state = WifiConfigState {
            active: active("old", "old-pass"),
            active_version: 7,
            pending: None,
            tx_state: WifiTxState::Committed,
        };
        assert!(!state.is_fresh_version(7));
        assert!(state.is_fresh_version(8));
    }

    fn provision_entry(
        ssid: &str,
        priority: u8,
        action: WifiSecretAction,
        password: Option<&str>,
    ) -> crate::WifiProvisionEntry {
        crate::WifiProvisionEntry {
            ssid: ssid.into(),
            priority,
            secret_action: action,
            password: password.map(|p| p.into()),
        }
    }

    fn provision_config(
        version: i64,
        entries: Vec<crate::WifiProvisionEntry>,
    ) -> crate::WifiProvisionConfig {
        crate::WifiProvisionConfig {
            config_version: version,
            entries,
        }
    }

    #[test]
    fn ota_with_keep_does_not_require_password() {
        let config = provision_config(
            8,
            vec![provision_entry("Farm-A", 0, WifiSecretAction::Keep, None)],
        );
        assert!(validate_provision_config(&config, 7).is_ok());
    }

    #[test]
    fn ota_with_set_requires_password() {
        let missing = provision_config(
            8,
            vec![provision_entry("Farm-A", 0, WifiSecretAction::Set, None)],
        );
        assert!(validate_provision_config(&missing, 7).is_err());

        let empty = provision_config(
            8,
            vec![provision_entry(
                "Farm-A",
                0,
                WifiSecretAction::Set,
                Some(""),
            )],
        );
        assert!(validate_provision_config(&empty, 7).is_err());

        let ok = provision_config(
            8,
            vec![provision_entry(
                "Farm-A",
                0,
                WifiSecretAction::Set,
                Some("secret"),
            )],
        );
        assert!(validate_provision_config(&ok, 7).is_ok());
    }

    #[test]
    fn ota_with_clear_requires_no_password() {
        let with_password = provision_config(
            8,
            vec![provision_entry(
                "Farm-A",
                0,
                WifiSecretAction::Clear,
                Some("secret"),
            )],
        );
        assert!(validate_provision_config(&with_password, 7).is_err());

        let clean = provision_config(
            8,
            vec![provision_entry("Farm-A", 0, WifiSecretAction::Clear, None)],
        );
        assert!(validate_provision_config(&clean, 7).is_ok());
    }

    #[test]
    fn invalid_wifi_version_is_rejected_before_ota() {
        let stale = provision_config(
            7,
            vec![provision_entry(
                "Farm-A",
                0,
                WifiSecretAction::Set,
                Some("secret"),
            )],
        );
        assert!(validate_provision_config(&stale, 7).is_err());
    }
    #[test]
    fn rejects_oversize_and_duplicate_provision_entries() {
        let too_many: Vec<_> = (0..9)
            .map(|i| provision_entry("Farm-A", i, WifiSecretAction::Keep, None))
            .collect();
        assert!(validate_provision_config(&provision_config(8, too_many), 7).is_err());

        let dup_priority = provision_config(
            8,
            vec![
                provision_entry("Farm-A", 0, WifiSecretAction::Keep, None),
                provision_entry("Farm-B", 0, WifiSecretAction::Keep, None),
            ],
        );
        assert!(validate_provision_config(&dup_priority, 7).is_err());

        let blank_ssid = provision_config(
            8,
            vec![provision_entry("   ", 0, WifiSecretAction::Keep, None)],
        );
        assert!(validate_provision_config(&blank_ssid, 7).is_err());
    }

    #[test]
    fn keep_resolves_password_from_active_list() {
        let active_list = active("Farm-A", "old-secret");
        let config = provision_config(
            8,
            vec![provision_entry("Farm-A", 0, WifiSecretAction::Keep, None)],
        );
        let resolved = resolve_provision_credentials(&config, &active_list).unwrap();
        assert_eq!(resolved.candidates[0].password, "old-secret");
    }

    #[test]
    fn keep_for_unknown_ssid_is_rejected() {
        let active_list = active("Farm-A", "old-secret");
        let config = provision_config(
            8,
            vec![provision_entry(
                "Farm-Unknown",
                0,
                WifiSecretAction::Keep,
                None,
            )],
        );
        assert!(resolve_provision_credentials(&config, &active_list).is_err());
    }

    #[test]
    fn clear_only_resolves_to_empty_and_is_rejected() {
        let active_list = active("Farm-A", "old-secret");
        let config = provision_config(
            8,
            vec![provision_entry("Farm-A", 0, WifiSecretAction::Clear, None)],
        );
        assert!(resolve_provision_credentials(&config, &active_list).is_err());
    }
}
