//! WiFi connection management with priority-ordered failover.

use esp_idf_hal::modem::Modem;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{AuthMethod, ClientConfiguration, Configuration, EspWifi};
use hydragrow_shared::WifiCandidate;
use log::{info, warn};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::{Duration, Instant};

use super::mqtt_client::ConnectionState;

pub fn connect_wifi(
    modem: Modem<'static>,
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
    mut candidates: Vec<WifiCandidate>,
    conn_tx: Sender<ConnectionState>,
) -> anyhow::Result<()> {
    candidates.retain(|candidate| !candidate.ssid.trim().is_empty());
    candidates.sort_by_key(|candidate| candidate.priority);
    anyhow::ensure!(!candidates.is_empty(), "No WiFi candidates are configured");

    let mut wifi = EspWifi::new(modem, sysloop, Some(nvs))?;
    wifi.start()?;

    thread::spawn(move || {
        let mut candidate_index = 0usize;
        let mut full_cycle_failures = 0u32;
        let mut was_connected = false;

        loop {
            let candidate = &candidates[candidate_index % candidates.len()];
            info!(
                "📡 [WIFI] Trying '{}' (priority {}).",
                candidate.ssid, candidate.priority
            );
            let configured = wifi.set_configuration(&Configuration::Client(ClientConfiguration {
                ssid: candidate.ssid.as_str().try_into().unwrap_or_default(),
                password: candidate.password.as_str().try_into().unwrap_or_default(),
                auth_method: AuthMethod::WPA2Personal,
                ..Default::default()
            }));

            let connected = configured.is_ok()
                && wifi.connect().is_ok()
                && wait_for_ip(&wifi, Duration::from_secs(15));
            if connected {
                info!("🌐 [WIFI] Connected to '{}'.", candidate.ssid);
                if !was_connected {
                    let _ = conn_tx.send(ConnectionState::WifiConnected);
                    was_connected = true;
                }
                while is_fully_connected(&wifi) {
                    thread::sleep(Duration::from_secs(2));
                }
                warn!(
                    "⚠️ [WIFI] Lost '{}'; trying the next candidate.",
                    candidate.ssid
                );
                if was_connected {
                    let _ = conn_tx.send(ConnectionState::WifiDisconnected);
                    was_connected = false;
                }
            } else {
                warn!("⚠️ [WIFI] Could not connect to '{}'.", candidate.ssid);
            }

            candidate_index += 1;
            if candidate_index % candidates.len() == 0 {
                full_cycle_failures = full_cycle_failures.saturating_add(1);
                warn!(
                    "⚠️ [WIFI] All configured networks failed (cycle {}).",
                    full_cycle_failures
                );
                thread::sleep(backoff_for(full_cycle_failures));
            }
        }
    });
    Ok(())
}

fn is_fully_connected(wifi: &EspWifi<'static>) -> bool {
    wifi.is_connected().unwrap_or(false)
        && wifi
            .sta_netif()
            .get_ip_info()
            .map(|info| !info.ip.is_unspecified())
            .unwrap_or(false)
}

fn wait_for_ip(wifi: &EspWifi<'static>, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if is_fully_connected(wifi) {
            return true;
        }
        thread::sleep(Duration::from_millis(500));
    }
    false
}

/// Exponential reconnect backoff, capped to avoid an unresponsive provisioning loop.
fn backoff_for(full_cycle_failures: u32) -> Duration {
    Duration::from_secs(2u64.saturating_pow(full_cycle_failures.min(5)).min(60))
}
