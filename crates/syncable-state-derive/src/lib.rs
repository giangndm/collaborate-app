mod attrs;
mod diagnostics;
mod expand;
mod parse;
mod schema;
mod snapshot;
mod validate;

use proc_macro::TokenStream;

#[proc_macro_derive(SyncableState, attributes(sync))]
pub fn derive_syncable_state(input: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(input as syn::DeriveInput);

    match expand::expand(input) {
        Ok(tokens) => tokens.into(),
        Err(error) => diagnostics::into_compile_error(error).into(),
    }
}
