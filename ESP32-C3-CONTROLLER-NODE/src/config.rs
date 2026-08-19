use hydragrow_shared::ControllerConfig;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CropStage {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ec_target: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ec_tolerance: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ph_target: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ph_tolerance: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub misting_on_duration_ms: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub misting_off_duration_ms: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControllerRuntimeState {
    pub base_config: ControllerConfig,
    pub effective_config: ControllerConfig,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_recipe: Option<CropStage>,
}

impl ControllerRuntimeState {
    pub fn new(base_config: ControllerConfig) -> Self {
        let mut state = Self {
            base_config: base_config.clone(),
            effective_config: base_config,
            active_recipe: None,
        };
        state.recompute_effective_config();
        state
    }

    pub fn set_base_config(&mut self, base_config: ControllerConfig) {
        self.base_config = base_config;
        self.recompute_effective_config();
    }

    pub fn set_active_recipe(&mut self, active_recipe: Option<CropStage>) {
        self.active_recipe = active_recipe;
        self.recompute_effective_config();
    }

    pub fn recompute_effective_config(&mut self) {
        self.effective_config = self.base_config.clone();
        if let Some(stage) = &self.active_recipe {
            apply_stage_override(&mut self.effective_config, stage);
        }
    }
}

impl Default for ControllerRuntimeState {
    fn default() -> Self {
        Self::new(ControllerConfig::default())
    }
}

pub type SharedConfig = Arc<RwLock<ControllerRuntimeState>>;

pub fn create_shared_config() -> SharedConfig {
    Arc::new(RwLock::new(ControllerRuntimeState::default()))
}

pub fn apply_stage_override(config: &mut ControllerConfig, stage: &CropStage) {
    if let Some(value) = stage.ec_target {
        config.ec_target = value;
    }
    if let Some(value) = stage.ec_tolerance {
        config.ec_tolerance = value;
    }
    if let Some(value) = stage.ph_target {
        config.ph_target = value;
    }
    if let Some(value) = stage.ph_tolerance {
        config.ph_tolerance = value;
    }
    if let Some(value) = stage.misting_on_duration_ms {
        config.misting_on_duration_ms = value;
    }
    if let Some(value) = stage.misting_off_duration_ms {
        config.misting_off_duration_ms = value;
    }
}
