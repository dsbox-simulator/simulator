use serde::{Deserialize, Serialize};
use crate::Payload;

#[derive(Payload, Serialize, Deserialize)]
pub struct Echo {
    pub echo: String,
}

#[derive(Payload, Serialize, Deserialize)]
pub struct EchoOk {
    pub echo: String,
}