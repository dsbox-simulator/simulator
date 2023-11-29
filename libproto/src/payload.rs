//! Trait to identify and deserialize message types
use serde::{Deserialize, Serialize};

/// This trait is implemented by all structs that are used in Messages body.
/// An implementation only has to provide the `type` string that is set in the message body
/// to identify itself. Then the Message should be able to be deserialized into the implementing
/// struct. This is of course only a convention and should not be taken as a guarantee.
/// Nodes may also define their own message types that have no corresponding struct that implements
/// [`Payload`].
pub trait Payload: Serialize + for<'de> Deserialize<'de> {
    /// The string that should be assigned to the `type` field of a message body to identify it as
    /// containing the data for a struct that implements [`Payload`].
    const TYPE: &'static str;
}