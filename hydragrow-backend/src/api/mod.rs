#![warn(clippy::unwrap_used)]

pub mod admin_users;
pub mod alert;
pub mod analytics;
pub mod calibration;
pub mod config;
pub mod config_backup;
pub mod control;
pub mod crop_season;
pub mod crop_season_photo;
pub mod device_admin;
pub mod device_pairing;
pub mod fleet;
pub mod health_topics;
pub mod metrics;
pub mod middleware;
pub mod mqtt_utils;
pub mod notification;
pub mod recipe;
pub mod scope_definitions;
pub mod script;
pub mod sensor;
pub mod solana;
pub mod webhook;
pub mod webhook_tokens;
pub mod ws;

#[cfg(test)]
pub(crate) mod test_support;
#[cfg(test)]
mod tests;
