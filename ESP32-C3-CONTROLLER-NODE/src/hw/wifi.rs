// src/hw/wifi.rs
//! Quản lý kết nối WiFi cho ESP32-C3.

use esp_idf_hal::modem::Modem;
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::wifi::{AuthMethod, ClientConfiguration, Configuration, EspWifi};
use log::{info, warn};
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

use super::mqtt_client::ConnectionState;

pub fn connect_wifi(
    modem: Modem<'static>, // 👈 Thêm lifetime 'static ở đây!
    sysloop: EspSystemEventLoop,
    nvs: EspDefaultNvsPartition,
    ssid: &str,
    pass: &str,
    conn_tx: Sender<ConnectionState>,
) -> anyhow::Result<()> {
    info!("📡 Đang cấu hình và kết nối WiFi (SSID: {})...", ssid);
    let mut wifi = EspWifi::new(modem, sysloop, Some(nvs))?;

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid.try_into().unwrap_or_default(),
        password: pass.try_into().unwrap_or_default(),
        auth_method: AuthMethod::WPA2Personal,
        ..Default::default()
    }))?;

    wifi.start()?;
    wifi.connect()?;

    // Thread giám sát kết nối WiFi - sở hữu EspWifi instance 'static
    thread::spawn(move || {
        let mut was_connected = false;
        loop {
            let is_l2_connected = wifi.is_connected().unwrap_or(false);
            let has_ip = wifi
                .sta_netif()
                .get_ip_info()
                .map(|info| !info.ip.is_unspecified())
                .unwrap_or(false);

            let is_fully_connected = is_l2_connected && has_ip;

            if is_fully_connected && !was_connected {
                info!("🌐 WiFi đã kết nối thành công và nhận được IP!");
                let _ = conn_tx.send(ConnectionState::WifiConnected);
                was_connected = true;
            } else if !is_fully_connected && was_connected {
                warn!("⚠️ Mất kết nối WiFi!");
                let _ = conn_tx.send(ConnectionState::WifiDisconnected);
                was_connected = false;
                if !is_l2_connected {
                    let _ = wifi.connect();
                }
            }
            thread::sleep(Duration::from_secs(2));
        }
    });

    Ok(())
}