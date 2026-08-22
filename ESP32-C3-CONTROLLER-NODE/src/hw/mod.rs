// src/hw/mod.rs
pub mod mqtt_client;
pub mod ntp;
pub mod nvs_store;
pub mod pcf857x;
pub mod pump_controller;
pub mod wifi;
pub mod wifi_store;

pub use mqtt_client::{
    create_shared_sensor_data, get_free_heap, get_uptime_ms, get_uptime_sec, get_wifi_rssi,
    init_mqtt_client, ConnectionState, SharedSensorData,
};
pub use ntp::sync_sntp_time;
pub use nvs_store::NvsStore;
pub use pump_controller::{PumpController, PumpType, WaterDirection};
pub use wifi::connect_wifi;
pub use wifi_store::{load_wifi_list, save_wifi_list};
pub mod ota;
pub use ota::{perform_ota_update, CURRENT_VERSION};
