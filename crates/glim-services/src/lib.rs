//! Blocking I/O services. Call from a background executor once the UI is running.

pub mod binary;
pub mod files;
pub mod git;
pub mod json;
pub mod media;
pub mod paged;
pub mod preferences;
pub mod preview;
pub mod safetensors;
pub mod watch;
pub mod workspace;
