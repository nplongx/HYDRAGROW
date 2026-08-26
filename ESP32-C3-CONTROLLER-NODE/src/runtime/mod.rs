// src/runtime/mod.rs
pub mod command_handler;
pub mod dispatcher;
pub mod fsm_loop;
pub mod health;
pub mod observers;

pub use health::build_status_msg;
