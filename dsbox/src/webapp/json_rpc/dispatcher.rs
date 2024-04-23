use std::collections::HashMap;
use std::io::{Read, Write};

use serde_json::Value;

use super::request::{Id, Request};
use super::response::{Error, INTERNAL_ERROR, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR, Response};

pub type RpcFn<C> = fn(C, Value) -> Result<Value, Error>;

pub type RpcFnNoContext = fn(Value) -> Result<Value, Error>;

pub struct JsonRpcDispatcher<C> {
    methods: HashMap<String, RpcFn<C>>,
    methods_no_context: HashMap<String, RpcFnNoContext>,
}

impl<C> JsonRpcDispatcher<C> {
    pub fn new() -> Self {
        Self { methods: HashMap::new(), methods_no_context: HashMap::new() }
    }

    pub fn register(&mut self, name: String, method: RpcFn<C>) {
        self.methods.insert(name, method);
    }

    pub fn register_no_context(&mut self, name: String, method: RpcFnNoContext) {
        self.methods_no_context.insert(name, method);
    }

    pub fn dispatch(&mut self, context: C, reader: impl Read, mut writer: impl Write) -> std::io::Result<()> {
        if let Some(response) = self.process(context, reader) {
            if let Err(e) = serde_json::to_writer(&mut writer, &response) {
                if let Some(io_error_kind) = e.io_error_kind() {
                    return Err(std::io::Error::new(io_error_kind, e));
                } else {
                    if let Err(e) = serde_json::to_writer(writer, &Response::error(response.id().clone(), INTERNAL_ERROR, format!("failed to serialize response: {e}"), None)) {
                        if let Some(io_error_kind) = e.io_error_kind() {
                            return Err(std::io::Error::new(io_error_kind, e));
                        } else {
                            panic!("failed to respond to client: {e}");
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn process(&mut self, context: C, reader: impl Read) -> Option<Response> {
        let parsed = match serde_json::from_reader(reader) {
            Ok(parsed) => parsed,
            Err(error) => return Some(Response::error(
                Id::Null,
                PARSE_ERROR,
                format!("error parsing the request: {error}"),
                None,
            )),
        };
        let request = match serde_json::from_value(parsed) {
            Ok(request) => request,
            Err(error) => return Some(Response::error(
                Id::Null,
                INVALID_REQUEST,
                format!("invalid request: {error}"),
                None,
            )),
        };
        self.process_request(context, request)
    }

    fn process_request(&mut self, context: C, request: Request) -> Option<Response> {
        if let Some(method) = self.methods.get(&request.method) {
            match method(context, request.params) {
                Ok(value) => Some(Response::ok(request.id?, value)),
                Err(e) => Some(Response::from_error(request.id?, e)),
            }
        } else if let Some(method) = self.methods_no_context.get(&request.method) {
            match method(request.params) {
                Ok(value) => Some(Response::ok(request.id?, value)),
                Err(e) => Some(Response::from_error(request.id?, e)),
            }
        } else {
            Some(Response::error(
                request.id?,
                METHOD_NOT_FOUND,
                format!("method not found: {}", request.method),
                None,
            ))
        }
    }
}