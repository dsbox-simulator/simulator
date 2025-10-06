use super::request::Id;
use serde::Serialize;
use serde_json::{Number, Value};

#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Response {
    Ok {
        jsonrpc: String,
        id: Id,
        result: Value,
    },
    Error {
        jsonrpc: String,
        id: Id,
        error: Error,
    },
}

impl Response {
    pub fn ok(id: Id, result: Value) -> Self {
        Self::Ok {
            jsonrpc: "2.0".to_string(),
            id,
            result,
        }
    }

    pub fn error(id: Id, code: impl Into<Number>, message: String, data: Option<Value>) -> Self {
        Self::Error {
            jsonrpc: "2.0".to_string(),
            id,
            error: Error {
                code: code.into(),
                message,
                data,
            },
        }
    }

    pub fn from_error(id: Id, error: Error) -> Self {
        Self::Error {
            jsonrpc: "2.0".to_string(),
            id,
            error,
        }
    }

    pub fn id(&self) -> &Id {
        match self {
            Response::Ok { id, .. } => id,
            Response::Error { id, .. } => id,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct Error {
    pub code: Number,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

pub const PARSE_ERROR: i64 = -32700;
pub const INVALID_REQUEST: i64 = -32600;
pub const METHOD_NOT_FOUND: i64 = -32601;
pub const INVALID_PARAMS: i64 = -32602;
pub const INTERNAL_ERROR: i64 = -32603;

impl Error {
    pub fn custom<T: Serialize>(message: String, data: Option<T>) -> Self {
        Self {
            code: (-32000).into(),
            message,
            data: data.map(|d| serde_json::to_value(d).unwrap()),
        }
    }
}
