//! Messages used to initialize nodes (only one for now)
use serde::{Deserialize, Serialize};

use crate::Payload;

/// Sent to a server when it is first started.
#[derive(Payload, Deserialize, Serialize)]
pub struct Init {
    /// the name of the server itself
    pub name: String,
    /// the name of the "core", useful for test that want to exchange system messages with the core
    pub core_name: String,
    /// the current crate version of the core. Useful for creating tests that require a specific version
    pub core_version: String,
    /// `true` if the receiving node is a test node.
    pub is_test: bool,
}
