// src/hw/mod.rs
pub mod mqtt_client;
pub mod ntp;
pub mod nvs_store;
pub mod pump_controller;
pub mod wifi;

pub use mqtt_client::{
    create_shared_sensor_data, get_free_heap, get_uptime_ms, get_uptime_sec, get_wifi_rssi,
    init_mqtt_client, ConnectionState, SharedSensorData,
};
pub use ntp::sync_sntp_time;
pub use nvs_store::NvsStore;
pub use pump_controller::{PumpController, PumpType, WaterDirection};
pub use wifi::connect_wifi;