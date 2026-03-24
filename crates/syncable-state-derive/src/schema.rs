use quote::quote;

use crate::parse::ParsedState;

pub fn expand_schema_expr(state: &ParsedState) -> proc_macro2::TokenStream {
    let fields = state
        .fields
        .iter()
        .filter(|field| field.is_included())
        .map(|field| {
            let wire_name = &field.wire_name;
            if field.attrs.is_id {
                quote! {
                    syncable_state::FieldSchema {
                        name: ::std::string::String::from(#wire_name),
                        kind: syncable_state::FieldKind::String,
                    }
                }
            } else {
                let ty = &field.ty;
                quote! {
                    syncable_state::FieldSchema {
                        name: ::std::string::String::from(#wire_name),
                        kind: __sync_field_kind::<#ty>(),
                    }
                }
            }
        });

    quote! {
        fn __sync_field_kind<T>() -> syncable_state::FieldKind
        where
            T: syncable_state::SyncableState,
        {
            let schema = T::schema();
            if schema.fields.len() == 1 && schema.fields[0].name == "root" {
                schema.fields[0].kind.clone()
            } else {
                syncable_state::FieldKind::Object
            }
        }

        syncable_state::StateSchema::new(vec![
            #(#fields,)*
        ])
    }
}
