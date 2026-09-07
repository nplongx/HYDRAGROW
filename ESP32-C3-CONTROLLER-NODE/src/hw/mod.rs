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
pub use nvs_store::DEVICE_ID_KEY;
pub use pump_controller::{PumpController, WaterDirection};
pub use wifi::connect_wifi;
pub use wifi_store::{
    commit_pending_wifi, count_active_ssids, get_active_wifi_version,
    load_active_wifi_list_from_nvs, load_pending_wifi_list, load_transaction_state, load_wifi_list,
    prepare_pending_wifi, rollback_pending_wifi, save_wifi_list,
};
pub mod ota;
pub use ota::CURRENT_VERSION;
