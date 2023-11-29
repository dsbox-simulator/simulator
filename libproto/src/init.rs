//! Messages used to initialize nodes (only one for now)
use serde::{Deserialize, Serialize};

use crate::Payload;

/// Sent to a server when it is first started.
#[derive(Payload, Serialize, Deserialize)]
pub struct Init {
    /// the name of the server itself
    pub name: String,
    /// the names of all running servers (including itself)
    /// TODO: is the order of this list somewhat deterministic?
    pub servers: Vec<String>,
}
