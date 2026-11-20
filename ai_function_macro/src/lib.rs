//! `#[ai_function]` captures a function's source, including its doc comments,
//! and turns the item into `fn name(&str) -> &'static str`.
//!
//! The agents send that string to the model as the function contract. The
//! original body is not executed; `println!(OUTPUT)` is part of the prompt.

use proc_macro::TokenStream;
use quote::quote;
use syn::ItemFn;

#[proc_macro_attribute]
pub fn ai_function(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let source = item.to_string();
    let function: ItemFn = match syn::parse(item) {
        Ok(function) => function,
        Err(err) => {
            let message = err.to_string();
            return quote! { compile_error!(#message); }.into();
        }
    };
    let name = function.sig.ident;
    let vis = function.vis;
    let expanded = quote! {
        #vis fn #name(_input: &str) -> &'static str {
            #source
        }
    };
    expanded.into()
}
