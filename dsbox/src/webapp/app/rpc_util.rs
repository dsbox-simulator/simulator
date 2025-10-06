use crate::webapp::app::App;
use crate::webapp::json_rpc::request::{Id, Request};
use crate::webapp::json_rpc::response::{
    Error, Response, INTERNAL_ERROR, INVALID_PARAMS, INVALID_REQUEST, METHOD_NOT_FOUND, PARSE_ERROR,
};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use std::future::Future;
use std::io::{Read, Write};
use std::pin::Pin;

pub(super) type RpcFn = Box<
    dyn for<'a> Fn(
            &'a mut App,
            Value,
        ) -> Pin<Box<dyn Future<Output = Result<Value, Error>> + Send + 'a>>
        + Send,
>;

impl App {
    pub async fn dispatch_raw(
        &mut self,
        reader: impl Read,
        mut writer: impl Write,
    ) -> std::io::Result<()> {
        if let Some(response) = self.process(reader).await {
            if let Err(e) = serde_json::to_writer(&mut writer, &response) {
                if let Some(io_error_kind) = e.io_error_kind() {
                    return Err(std::io::Error::new(io_error_kind, e));
                } else {
                    if let Err(e) = serde_json::to_writer(
                        writer,
                        &Response::error(
                            response.id().clone(),
                            INTERNAL_ERROR,
                            format!("failed to serialize response: {e}"),
                            None,
                        ),
                    ) {
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

    async fn process(&mut self, reader: impl Read) -> Option<Response> {
        let parsed = match serde_json::from_reader(reader) {
            Ok(parsed) => parsed,
            Err(error) => {
                return Some(Response::error(
                    Id::Null,
                    PARSE_ERROR,
                    format!("error parsing the request: {error}"),
                    None,
                ))
            }
        };
        let request = match serde_json::from_value(parsed) {
            Ok(request) => request,
            Err(error) => {
                return Some(Response::error(
                    Id::Null,
                    INVALID_REQUEST,
                    format!("invalid request: {error}"),
                    None,
                ))
            }
        };
        self.process_request(request).await
    }

    async fn process_request(&mut self, request: Request) -> Option<Response> {
        if let Some(rpc_fn) = Self::rpc_dispatch_map(&request.method) {
            let future = rpc_fn(self, request.params);
            match future.await {
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

    pub(super) async fn dispatch_with_args<'a, A, R, F>(
        &'a mut self,
        args: Value,
        inner: impl Fn(&'a mut Self, A) -> F,
    ) -> Result<Value, Error>
    where
        A: DeserializeOwned,
        R: Serialize,
        F: Future<Output = Result<R, Error>> + 'a,
    {
        let args: A = match serde_json::from_value(args) {
            Ok(args) => Ok(args),
            Err(e) => Err(Error {
                code: INVALID_PARAMS.into(),
                message: format!("failed to deserialize method parameters: {e}"),
                data: None,
            }),
        }?;
        let result = inner(self, args).await?;
        match serde_json::to_value(result) {
            Ok(result) => Ok(result),
            Err(e) => Err(Error {
                code: INTERNAL_ERROR.into(),
                message: format!("failed to serialize method response: {e}"),
                data: None,
            }),
        }
    }
}
