//! Multi-device Digital Twin runtime.

use crate::harness::Harness;
use anyhow::{Result, bail};
use hydragrow_shared::ControllerConfig;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TwinDeviceIdentity {
    pub device_id: String,
    pub station_id: String,
    pub firmware_version: String,
    pub capabilities: Vec<String>,
}

pub struct DigitalTwinRuntime {
    devices: BTreeMap<String, (TwinDeviceIdentity, Harness)>,
}

impl Default for DigitalTwinRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl DigitalTwinRuntime {
    pub fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
        }
    }

    pub fn add_device(
        &mut self,
        identity: TwinDeviceIdentity,
        config: ControllerConfig,
        tank: crate::plant::tank::Tank,
    ) -> Result<()> {
        if self.devices.contains_key(&identity.device_id) {
            bail!("duplicate digital twin device_id: {}", identity.device_id);
        }
        let harness = Harness::builder(config, tank)
            .device_id(identity.device_id.clone())
            .build()?;
        self.devices
            .insert(identity.device_id.clone(), (identity, harness));
        Ok(())
    }

    pub fn device(&self, device_id: &str) -> Option<&Harness> {
        self.devices.get(device_id).map(|(_, harness)| harness)
    }

    pub fn device_mut(&mut self, device_id: &str) -> Option<&mut Harness> {
        self.devices.get_mut(device_id).map(|(_, harness)| harness)
    }

    pub fn identity(&self, device_id: &str) -> Option<&TwinDeviceIdentity> {
        self.devices.get(device_id).map(|(identity, _)| identity)
    }

    pub fn tick_all(&mut self, dt_ms: u64) -> Result<()> {
        for (_, harness) in self.devices.values_mut() {
            harness.tick(dt_ms)?;
        }
        Ok(())
    }

    pub fn len(&self) -> usize {
        self.devices.len()
    }
    pub fn is_empty(&self) -> bool {
        self.devices.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plant::tank::Tank;

    #[test]
    fn runtime_manages_multiple_independent_twins() {
        let mut runtime = DigitalTwinRuntime::new();
        for device_id in ["sim-a", "sim-b"] {
            runtime
                .add_device(
                    TwinDeviceIdentity {
                        device_id: device_id.into(),
                        station_id: "station-1".into(),
                        firmware_version: "digital-twin".into(),
                        capabilities: vec![
                            "controller".into(),
                            "sensors".into(),
                            "actuators".into(),
                        ],
                    },
                    ControllerConfig::default(),
                    Tank::default(),
                )
                .unwrap();
        }
        runtime.tick_all(1000).unwrap();
        assert_eq!(runtime.len(), 2);
        assert_eq!(runtime.device("sim-a").unwrap().uptime_ms(), 1000);
        assert_eq!(runtime.device("sim-b").unwrap().uptime_ms(), 1000);
    }
}
