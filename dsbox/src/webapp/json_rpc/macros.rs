macro_rules! json_rpc {
    ($v:vis fn $name:ident($([$cn:ident: $ct:ty],)? $($n:ident: $t:ty),*) $(-> $ret:ty)? {$($body:tt)*}) => {
        $v fn $name($($cn: $ct,)? $($n: $t),*) $(-> $ret)? {
            $($body)*
        }

        mod $name {
            use crate::response::{Error, INTERNAL_ERROR, INVALID_PARAMS};

            #[derive(serde::Deserialize)]
            struct Args {
                $($n: $t),*
            }

            fn dispatch($($cn: $ct,)? args: Args) $(-> $ret)? {
                super::$name($($cn,)? $(args.$n),*)
            }

            #[allow(unused)]
            pub fn rpc_call($($cn: $ct,)? args: serde_json::Value) -> Result<serde_json::Value, Error> {
                let args = match serde_json::from_value(args) {
                    Ok(args) => args,
                    Err(e) => {
                        return Err(Error {
                            code: INVALID_PARAMS.into(),
                            message: format!("failed to deserialize method parameters: {e}"),
                            data: None,
                        });
                    }
                };
                let result = dispatch($($cn,)? args);
                let result = match serde_json::to_value(result) {
                    Ok(result) => result,
                    Err(e) => {
                        return Err(Error {
                            code: INTERNAL_ERROR.into(),
                            message: format!("failed to serialize method response: {e}"),
                            data: None,
                        });
                    }
                };
                Ok(result)
            }
        }
    };

}

pub(crate) use {json_rpc};