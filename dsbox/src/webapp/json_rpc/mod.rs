//! a simple implementation of a json_rpc dispatcher

pub use dispatcher::{JsonRpcDispatcher, RpcFn};

pub mod request;
pub mod response;
mod dispatcher;