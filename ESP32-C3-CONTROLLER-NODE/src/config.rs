#[allow(unused_imports)]
pub use hydragrow_controller_core::core::fsm::recipe_manager::{
    apply_stage_override, ControllerRuntimeState,
};
use std::sync::{Arc, RwLock};

pub type SharedConfig = Arc<RwLock<ControllerRuntimeState>>;

pub fn create_shared_config() -> SharedConfig {
    Arc::new(RwLock::new(ControllerRuntimeState::default()))
}

