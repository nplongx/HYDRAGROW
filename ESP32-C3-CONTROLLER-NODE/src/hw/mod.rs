// src/hw/mod.rs
pub mod captive_portal;
pub mod mqtt_client;
pub mod ntp;
pub use captive_portal::run_captive_portal;

pub mod nvs_store;
pub mod pcf857x;
pub mod pump_controller;
pub mod wifi;
pub mod wifi_store;

pub use mqtt_client::create_shared_sensor_data;
pub use ntp::sync_sntp_time;
pub use nvs_store::NvsStore;
pub use pump_controller::PumpController;
pub use wifi::connect_wifi;
pub use wifi_store::{load_wifi_list, save_wifi_list};
pub mod ota;
pub use ota::CURRENT_VERSION;
