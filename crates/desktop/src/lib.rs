//! Aideon Tauri host entrypoint and IPC commands.

mod app;
mod commands;
mod health;
mod ipc;
mod menu;
mod mneme;
mod praxis_api;
mod scene;
mod setup;
mod temporal;
mod windows;
mod worker;
mod workspace;

pub use aideon_praxis::mneme::WorkerHealth;
pub use aideon_praxis::praxis::temporal::{DiffArgs, DiffSummary, StateAtArgs, StateAtResult};
pub use ipc::HostError;

pub fn run() {
    app::run();
}
