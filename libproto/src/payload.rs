use serde::{Deserialize, Serialize};

pub trait Payload: Serialize + for<'de> Deserialize<'de> {
    const TYPE: &'static str;
}