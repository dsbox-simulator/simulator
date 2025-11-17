//! Simple derive macro to help implementing the Payload trait from libproto.
//!
//! i.e. for
//! ```
//! #[derive(Payload)]
//! struct Init {/*...*/}
//! ```
//!
//! it simply generates
//! ```
//! impl Payload for Init {
//!    const TYPE: &'static str = "init";
//! }
//! ```
//!
//! this is used to identify message body types when deserializing messages.

use quote::quote;
use syn::{parse_macro_input, DeriveInput};
use heck::ToSnakeCase;

#[proc_macro_derive(Payload)]
pub fn derive_trait(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let name = input.ident;
    let ty = name.to_string().to_snake_case();

    let expanded = quote! {
        impl Payload for #name {
            const TYPE: &'static str = #ty;
        }
    };

    proc_macro::TokenStream::from(expanded)
}