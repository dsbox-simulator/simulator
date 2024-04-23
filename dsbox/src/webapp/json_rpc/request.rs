use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Number, Value};
use serde::de::{Error, Unexpected};

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum Id {
    String(String),
    Number(Number),
    Null,
}

#[derive(Debug, Deserialize)]
pub struct Request {
    #[serde(deserialize_with = "deserialize_jsonrpc_version")]
    #[allow(unused)]
    jsonrpc: String,
    pub method: String,
    pub params: Value,
    #[serde(default, deserialize_with = "deserialize_request_id")]
    pub id: Option<Id>,
}

// Any value that is present is considered Some value, including null.
fn deserialize_request_id<'de, D>(deserializer: D) -> Result<Option<Id>, D::Error>
    where D: Deserializer<'de>
{
    Deserialize::deserialize(deserializer).map(Some)
}

// only "2.0" allowed
fn deserialize_jsonrpc_version<'de, D>(deserializer: D) -> Result<String, D::Error>
    where D: Deserializer<'de>
{
    let version: String = Deserialize::deserialize(deserializer)?;
    if version != "2.0" {
        return Err(Error::invalid_value(Unexpected::Str(&version), &"2.0"));
    };
    Ok(version)
}
