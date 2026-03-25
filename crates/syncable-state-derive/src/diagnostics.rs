pub fn combine(errors: Vec<syn::Error>) -> syn::Result<()> {
    let mut errors = errors.into_iter();
    let Some(mut error) = errors.next() else {
        return Ok(());
    };

    for next in errors {
        error.combine(next);
    }

    Err(error)
}

pub fn into_compile_error(error: syn::Error) -> proc_macro2::TokenStream {
    error.into_compile_error()
}
