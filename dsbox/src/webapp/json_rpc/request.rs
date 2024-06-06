use serde::{Deserialize, Deserializer, Serialize};
use serde::de::{Error, Unexpected};
use serde_json::{Number, Value};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Id {
    String(String),
    Number(Number),
    Null,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Request {
    #[serde(deserialize_with = "deserialize_jsonrpc_version")]
    #[allow(unused)]
    jsonrpc: String,
    pub method: String,
    pub params: Value,
    #[serde(default, deserialize_with = "deserialize_request_id")]
    pub id: Option<Id>,
}

impl Request {
    pub fn new(method: String, params: impl Serialize, id: Id) -> Result<Request, serde_json::Error> {
        Ok(Self {
            jsonrpc: "2.0".to_string(),
            method,
            params: serde_json::to_value(params)?,
            id: Some(id),
        })
    }

    pub fn notification(method: String, params: impl Serialize) -> Result<Request, serde_json::Error> {
        Ok(Self {
            jsonrpc: "2.0".to_string(),
            method,
            params: serde_json::to_value(params)?,
            id: None,
        })
    }
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
