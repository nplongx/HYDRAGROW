//! Persistence for remotely provisioned WiFi credentials.

use anyhow::Result;
use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition, EspNvs};
use hydragrow_shared::WifiCredentialList;
use log::{info, warn};

const WIFI_LIST_KEY: &str = "wifi_list";
const WIFI_LIST_BUF_SIZE: usize = 2048;

/// NVS keys for the transactional pending/active WiFi state.
/// Active credentials stay under `wifi_list`; pending data lives under
/// separate keys so `prepare` never replaces the active list.
pub const WIFI_PENDING_KEY: &str = "wifi_pending";
pub const WIFI_PENDING_VERSION_KEY: &str = "wifi_p_ver";
pub const WIFI_PENDING_TARGET_KEY: &str = "wifi_p_target";
pub const WIFI_ACTIVE_VERSION_KEY: &str = "wifi_a_ver";
pub const WIFI_TRANSACTION_STATE_KEY: &str = "wifi_tx_state";
const WIFI_PENDING_BUF_SIZE: usize = 2048;

pub fn load_wifi_list(nvs_partition: EspDefaultNvsPartition) -> WifiCredentialList {
    let Ok(nvs) = EspNvs::new(nvs_partition, "agitech", true) else {
        return WifiCredentialList::default();
    };
    let mut buffer = [0u8; WIFI_LIST_BUF_SIZE];
    match nvs.get_str(WIFI_LIST_KEY, &mut buffer) {
        Ok(Some(raw)) => match serde_json::from_str::<WifiCredentialList>(raw) {
            Ok(list) => {
                info!(
                    "📶 [WIFI] Restored {} configured SSIDs from NVS.",
                    list.candidates.len()
                );
                list
            }
            Err(error) => {
                warn!(
                    "📶 [WIFI] Ignoring invalid persisted WiFi list: {:?}",
                    error
                );
                WifiCredentialList::default()
            }
        },
        _ => WifiCredentialList::default(),
    }
}

pub fn save_wifi_list(nvs: &mut EspDefaultNvs, list: &WifiCredentialList) -> Result<()> {
    nvs.set_str(WIFI_LIST_KEY, &serde_json::to_string(list)?)?;
    Ok(())
}

/// Stage a pending WiFi config without touching the active `wifi_list`.
/// Write order: pending config → pending metadata/version → tx state.
pub fn prepare_pending_wifi(
    nvs: &mut EspDefaultNvs,
    config: &WifiCredentialList,
    version: i64,
) -> Result<()> {
    nvs.set_str(WIFI_PENDING_KEY, &serde_json::to_string(config)?)?;
    nvs.set_i64(WIFI_PENDING_VERSION_KEY, version)?;
    nvs.set_str(WIFI_TRANSACTION_STATE_KEY, "Prepared")?;
    Ok(())
}

/// Promote the staged pending config to active.
pub fn commit_pending_wifi(nvs: &mut EspDefaultNvs) -> Result<()> {
    let mut buffer = [0u8; WIFI_PENDING_BUF_SIZE];
    let pending: WifiCredentialList = match nvs.get_str(WIFI_PENDING_KEY, &mut buffer)? {
        Some(raw) => serde_json::from_str(raw)?,
        None => anyhow::bail!("no pending wifi config to commit"),
    };
    let version = nvs.get_i64(WIFI_PENDING_VERSION_KEY)?.unwrap_or_default();
    save_wifi_list(nvs, &pending)?;
    nvs.set_i64(WIFI_ACTIVE_VERSION_KEY, version)?;
    nvs.remove(WIFI_PENDING_KEY)?;
    nvs.remove(WIFI_PENDING_VERSION_KEY)?;
    nvs.remove(WIFI_PENDING_TARGET_KEY)?;
    nvs.set_str(WIFI_TRANSACTION_STATE_KEY, "Committed")?;
    Ok(())
}

/// Discard the staged pending config; active credentials are untouched.
pub fn rollback_pending_wifi(nvs: &mut EspDefaultNvs) -> Result<()> {
    nvs.remove(WIFI_PENDING_KEY)?;
    nvs.remove(WIFI_PENDING_VERSION_KEY)?;
    nvs.remove(WIFI_PENDING_TARGET_KEY)?;
    nvs.set_str(WIFI_TRANSACTION_STATE_KEY, "Committed")?;
    Ok(())
}

/// Load the staged pending WiFi config, if any.
pub fn load_pending_wifi_list(nvs_partition: EspDefaultNvsPartition) -> Option<WifiCredentialList> {
    let nvs = EspNvs::new(nvs_partition, "agitech", true).ok()?;
    let mut buffer = [0u8; WIFI_PENDING_BUF_SIZE];
    match nvs.get_str(WIFI_PENDING_KEY, &mut buffer) {
        Ok(Some(raw)) => match serde_json::from_str::<WifiCredentialList>(raw) {
            Ok(list) => {
                info!(
                    "📶 [WIFI] Found {} pending SSIDs in NVS.",
                    list.candidates.len()
                );
                Some(list)
            }
            Err(error) => {
                warn!("📶 [WIFI] Ignoring invalid pending WiFi list: {:?}", error);
                None
            }
        },
        _ => None,
    }
}

/// Read the active WiFi list via an already-open NVS handle (for Keep resolution).
pub fn load_active_wifi_list_from_nvs(nvs: &mut EspDefaultNvs) -> WifiCredentialList {
    let mut buffer = [0u8; WIFI_LIST_BUF_SIZE];
    match nvs.get_str(WIFI_LIST_KEY, &mut buffer) {
        Ok(Some(raw)) => serde_json::from_str::<WifiCredentialList>(raw).unwrap_or_default(),
        _ => WifiCredentialList::default(),
    }
}

/// Read the active WiFi config version (defaults to 0 when unset).
pub fn get_active_wifi_version(nvs: &mut EspDefaultNvs) -> i64 {
    nvs.get_i64(WIFI_ACTIVE_VERSION_KEY)
        .unwrap_or(None)
        .unwrap_or_default()
}

/// Count active SSIDs without exposing secrets (for status payloads).
pub fn count_active_ssids(nvs: &mut EspDefaultNvs) -> usize {
    let mut buffer = [0u8; WIFI_LIST_BUF_SIZE];
    match nvs.get_str(WIFI_LIST_KEY, &mut buffer) {
        Ok(Some(raw)) => serde_json::from_str::<WifiCredentialList>(raw)
            .map(|list| list.sorted_valid().len())
            .unwrap_or_default(),
        _ => 0,
    }
}

/// Load the persisted transaction marker; defaults to `Committed`.
pub fn load_transaction_state(nvs_partition: EspDefaultNvsPartition) -> String {
    let Ok(nvs) = EspNvs::new(nvs_partition, "agitech", true) else {
        return "Committed".to_string();
    };
    let mut buffer = [0u8; 32];
    match nvs.get_str(WIFI_TRANSACTION_STATE_KEY, &mut buffer) {
        Ok(Some(state)) => state.to_string(),
        _ => "Committed".to_string(),
    }
}
