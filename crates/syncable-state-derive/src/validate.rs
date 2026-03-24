use std::collections::BTreeMap;

use crate::parse::ParsedState;

pub fn validate_state(state: &ParsedState) -> syn::Result<()> {
    let mut errors = Vec::new();
    let mut id_field = None;
    let mut wire_names = BTreeMap::<&str, &syn::Ident>::new();

    for field in &state.fields {
        if field.attrs.is_id
            && let Some(existing) = id_field.replace(&field.ident) {
                errors.push(syn::Error::new(
                    field.ident.span(),
                    format!(
                        "duplicate #[sync(id)] fields: `{}` and `{}`",
                        existing, field.ident
                    ),
                ));
            }

        if field.attrs.is_id && field.attrs.is_skip {
            errors.push(syn::Error::new(
                field.ident.span(),
                "a field cannot use both #[sync(id)] and #[sync(skip)]",
            ));
        }

        if field.attrs.with.is_some() {
            errors.push(syn::Error::new(
                field.ident.span(),
                "#[sync(with = ...)] is not supported in v1",
            ));
        }

        if !field.is_included() {
            continue;
        }

        if let Some(existing) = wire_names.insert(&field.wire_name, &field.ident) {
            errors.push(syn::Error::new(
                field.ident.span(),
                format!(
                    "wire name collision for `{}` between fields `{}` and `{}`",
                    field.wire_name, existing, field.ident
                ),
            ));
        }
    }

    crate::diagnostics::combine(errors)
}
