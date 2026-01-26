pub mod client;
pub mod commands;
pub mod config;
pub mod error;
pub mod output;

pub use client::Client as SlackClient;
pub use config::Config;
pub use error::{Result, SlackCliError};
pub use output::Output;
