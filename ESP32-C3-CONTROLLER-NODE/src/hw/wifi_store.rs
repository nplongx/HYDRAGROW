//! Persistence for remotely provisioned WiFi credentials.

use anyhow::Result;
use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition, EspNvs};
use hydragrow_shared::WifiCredentialList;
use log::{info, warn};

const WIFI_LIST_KEY: &str = "wifi_list";
const WIFI_LIST_BUF_SIZE: usize = 2048;

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
