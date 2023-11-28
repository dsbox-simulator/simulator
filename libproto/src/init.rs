use serde::{Deserialize, Serialize};

use crate::Payload;

#[derive(Payload, Serialize, Deserialize)]
pub struct Init {
    pub name: String,
    pub servers: Vec<String>,
}

#[derive(Payload, Serialize, Deserialize)]
pub struct Start {}
