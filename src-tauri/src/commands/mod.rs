//! Tauri commands for liscov

pub mod auth;
pub mod auth_window;
pub mod chat;
pub mod config;
pub mod websocket;
pub mod database;
pub mod analytics;
pub mod tts;
pub mod viewer;
pub mod raw_response;

// Re-export all commands for easy registration
pub use auth::*;
pub use chat::*;
pub use config::*;
pub use websocket::*;
pub use database::*;
pub use analytics::*;
pub use tts::*;
pub use viewer::*;
pub use raw_response::*;
