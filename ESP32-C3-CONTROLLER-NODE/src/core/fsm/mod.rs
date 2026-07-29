// src/core/fsm/mod.rs
//! Core FSM Engine Module.
//! Thuộc tầng Pure Core: Không chứa thread execution hay phần cứng ESP-IDF.

pub mod context;
pub mod events;
pub mod orchestrator;
pub mod peripheral;
pub mod phase_tick;
pub mod phases;
pub mod tick_result;
pub mod types;

pub use context::SystemContext;
pub use events::{DosingPumpTarget, OrchestratorEvent};
pub use phase_tick::PhaseTick;
pub use tick_result::{ContextDelta, PeripheralDelta, TickResult};
pub use types::SharedSensorData;