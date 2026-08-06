pub mod api;
pub mod benchmark;
pub mod browser;
pub mod config;
pub mod daemon;
pub mod health;
pub mod metrics;
pub mod permission;
pub mod server;

// India Stack is optional (feature-gated for regional availability)
#[cfg(feature = "india_stack")]
pub mod india_stack;
