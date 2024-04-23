//! a simple implementation of a json_rpc dispatcher

pub use dispatcher::{JsonRpcDispatcher, RpcFn};
pub(crate) use macros::json_rpc;

mod macros;
mod request;
mod response;
mod dispatcher;