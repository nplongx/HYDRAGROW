//! Pure host-testable state model for transactional WiFi provisioning.
//!
//! The ESP32 NVS layer (`wifi_store.rs`) persists these transitions; this
//! module defines the semantics so they can be unit-tested on the host.

use serde::{Deserialize, Serialize};

use crate::{WifiCandidate, WifiCredentialList};

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

/// Convenience constructors for tests.
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
}
