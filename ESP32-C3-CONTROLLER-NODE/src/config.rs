use hydragrow_shared::ControllerConfig;
use std::sync::{Arc, RwLock};

pub type SharedConfig = Arc<RwLock<ControllerConfig>>;

pub fn create_shared_config() -> SharedConfig {
    Arc::new(RwLock::new(ControllerConfig::default()))
}
