use anyhow::{anyhow, Result};
use esp_idf_svc::nvs::{EspDefaultNvsPartition, EspNvs, NvsDefault};
use esp_idf_svc::wifi::{BlockingWifi, EspWifi};
use log::{info, warn};
use serde::{Deserialize, Serialize};

const NVS_NAMESPACE: &str = "agitech";
const NVS_KEY: &str = "wifi_list";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WifiCandidate {
    pub ssid: String,
    pub password: String,
    pub priority: u8,
}

pub struct WifiProvisioner {
    nvs: EspNvs<NvsDefault>,
    fallback_ssid: String,
    fallback_pass: String,
}

impl WifiProvisioner {
    pub fn new(
        nvs_partition: EspDefaultNvsPartition,
        fallback_ssid: &str,
        fallback_pass: &str,
    ) -> Result<Self> {
        let nvs = EspNvs::new(nvs_partition, NVS_NAMESPACE, true)?;
        Ok(Self {
            nvs,
            fallback_ssid: fallback_ssid.to_string(),
            fallback_pass: fallback_pass.to_string(),
        })
    }

    /// Load candidates từ NVS, sort by priority DESC.
    /// Nếu NVS rỗng, trả về fallback từ secrets.
    pub fn load(&self) -> Vec<WifiCandidate> {
        // Đọc JSON string từ NVS
        let mut buf = [0u8; 1024];
        match self.nvs.get_raw(NVS_KEY, &mut buf) {
            Ok(Some(bytes)) => {
                if let Ok(s) = std::str::from_utf8(bytes) {
                    if let Ok(mut candidates) = serde_json::from_str::<Vec<WifiCandidate>>(s) {
                        candidates.sort_by(|a, b| b.priority.cmp(&a.priority));
                        return candidates;
                    }
                }
            }
            _ => {}
        }
        // Fallback
        if !self.fallback_ssid.is_empty() {
            vec![WifiCandidate {
                ssid: self.fallback_ssid.clone(),
                password: self.fallback_pass.clone(),
                priority: 0,
            }]
        } else {
            vec![]
        }
    }

    /// Save candidates lên NVS.
    pub fn save(&mut self, candidates: &[WifiCandidate]) -> Result<()> {
        let json = serde_json::to_string(candidates)?;
        self.nvs
            .set_raw(NVS_KEY, json.as_bytes())
            .map_err(|e| anyhow!("NVS write error: {:?}", e))?;
        Ok(())
    }
}

/// Thử kết nối WiFi theo danh sách candidates (blocking).
/// Trả về Ok(()) khi kết nối thành công, Err nếu hết candidates.
pub fn connect_wifi(
    wifi: &mut BlockingWifi<EspWifi<'static>>,
    candidates: &[WifiCandidate],
) -> Result<()> {
    use esp_idf_svc::wifi::{ClientConfiguration, Configuration};

    for candidate in candidates {
        info!("Đang kết nối WiFi: {}", candidate.ssid);

        wifi.set_configuration(&Configuration::Client(ClientConfiguration {
            ssid: candidate.ssid.as_str().try_into().unwrap_or_default(),
            password: candidate.password.as_str().try_into().unwrap_or_default(),
            ..Default::default()
        }))?;

        wifi.start()?;

        match wifi.connect() {
            Ok(_) => {
                wifi.wait_netif_up()?;
                info!("WiFi kết nối thành công: {}", candidate.ssid);
                return Ok(());
            }
            Err(e) => {
                warn!("WiFi thất bại với '{}': {:?}", candidate.ssid, e);
                let _ = wifi.disconnect();
                let _ = wifi.stop();
            }
        }
    }
    Err(anyhow!("Không kết nối được WiFi nào!"))
}
