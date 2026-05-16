//! Tauri commands for liscov

pub mod analytics;
pub mod auth;
pub mod auth_window;
pub mod chat;
pub mod config;
pub mod database;
pub mod raw_response;
pub mod tts;
pub mod viewer;
pub mod websocket;

// Re-export all commands for easy registration
pub use analytics::*;
pub use auth::*;
pub use chat::*;
pub use config::*;
pub use database::*;
pub use raw_response::*;
pub use tts::*;
pub use viewer::*;
pub use websocket::*;
