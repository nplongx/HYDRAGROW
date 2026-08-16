// src/runtime/mod.rs
pub mod command_handler;
pub mod dispatcher;
pub mod fsm_loop;
pub mod health;
pub mod observers;

pub use command_handler::process_mqtt_commands;
pub use dispatcher::{DispatchContext, EventDispatcher};
pub use fsm_loop::start_fsm_control_loop;
pub use health::{build_status_msg, hestia_action_from_phase, run_main_health_loop};
