use quote::quote;
use syn::{FnArg, ItemFn, parse, ReturnType};

#[proc_macro_attribute]
pub fn json_rpc(_attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let item = parse::<ItemFn>(item).unwrap();

    let name = &item.sig.ident;

    let return_type = match &item.sig.output {
        ReturnType::Default => quote! { () },
        ReturnType::Type(_, ty) => quote! {#ty}
    };

    let mut rpc_args = Vec::new();
    let mut rpc_params = Vec::new();

    let mut context_args = Vec::new();
    let mut context_params = Vec::new();

    let mut context_ty = quote! {()};
    for (idx, arg) in item.sig.inputs.iter().enumerate() {
        let FnArg::Typed(arg) = arg else { unimplemented!(); };
        if idx == 0 {
            context_args.push(arg);
            context_params.push(&arg.pat);
            let syn::Type::Reference(ty) = &*arg.ty else { unimplemented!(); };
            let actual_ty = &ty.elem;
            context_ty = quote! {#actual_ty};
        } else {
            rpc_args.push(arg);
            rpc_params.push(&arg.pat);
        }
    }


    let token_stream = quote! {
        #item

        mod #name {
            use super::*;
            use crate::webapp::json_rpc::response::{Error, INTERNAL_ERROR, INVALID_PARAMS};

            #[derive(serde::Deserialize)]
            struct Args {
                #(#rpc_args),*
            }

            async fn dispatch(#(#context_args),*, args: Args) -> #return_type {
                super::#name(#(#context_params),*, #(args.#rpc_params),*).await
            }

            async fn rpc_call(#(#context_args),*, args: serde_json::Value) -> Result<serde_json::Value, Error> {
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
                let result = dispatch(#(#context_params),*, args).await;
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

            pub fn register(dispatcher: &mut crate::webapp::json_rpc::JsonRpcDispatcher<#context_ty>) {
                dispatcher.register(stringify!(#name).to_string(), |#(#context_args),*,args: serde_json::Value| Box::pin(rpc_call(#(#context_params),*, args)));
            }
        }
    };

    proc_macro::TokenStream::from(token_stream)
}