//! Aideon Tauri host entrypoint and IPC commands.

mod app;
mod commands;
mod health;
mod menu;
mod mneme;
mod praxis_api;
mod scene;
mod setup;
mod temporal;
mod windows;
mod worker;

pub use aideon_praxis_facade::mneme::WorkerHealth;
pub use aideon_praxis_facade::praxis::temporal::{
    DiffArgs, DiffSummary, StateAtArgs, StateAtResult,
};

pub fn run() {
    app::run();
}
