//! Blocking I/O services. Call from a background executor once the UI is running.

pub mod files;
pub mod git;
pub mod workspace;

pub mod watch;

pub mod media;

pub mod binary;

pub mod preferences;

pub mod paged;
pub mod safetensors;
