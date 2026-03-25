use quote::{format_ident, quote};

use crate::parse::ParsedState;

pub fn snapshot_ident(state: &ParsedState) -> syn::Ident {
    format_ident!("{}Snapshot", state.ident)
}

pub fn expand_snapshot(state: &ParsedState) -> proc_macro2::TokenStream {
    let snapshot_ident = snapshot_ident(state);
    let vis = &state.vis;
    let generics = crate::expand::snapshot_generics(state);
    let (impl_generics, _ty_generics, where_clause) = generics.split_for_impl();

    let fields = state
        .fields
        .iter()
        .filter(|field| field.is_included())
        .map(|field| {
            let ident = &field.ident;
            if field.attrs.is_id {
                quote! { #ident: ::std::string::String }
            } else {
                let ty = &field.ty;
                quote! { #ident: <#ty as syncable_state::SyncableState>::Snapshot }
            }
        });

    quote! {
        #[derive(Clone, PartialEq)]
        #vis struct #snapshot_ident #impl_generics #where_clause {
            #(pub #fields,)*
        }
    }
}

pub fn expand_snapshot_expr(state: &ParsedState) -> proc_macro2::TokenStream {
    let snapshot_ident = snapshot_ident(state);
    let (_, ty_generics, _) = state.generics.split_for_impl();
    let fields = state
        .fields
        .iter()
        .filter(|field| field.is_included())
        .map(|field| {
            let ident = &field.ident;
            if field.attrs.is_id {
                quote! { #ident: ::std::string::ToString::to_string(&self.#ident) }
            } else {
                quote! { #ident: syncable_state::SyncableState::snapshot(&self.#ident) }
            }
        });

    quote! {
        #snapshot_ident #ty_generics {
            #(#fields,)*
        }
    }
}

pub fn expand_snapshot_codec(state: &ParsedState) -> proc_macro2::TokenStream {
    let ident = &state.ident;
    let generics = crate::expand::impl_generics(state);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let to_value_fields = state.fields.iter().filter(|field| field.is_included()).map(|field| {
        let field_ident = &field.ident;
        let wire_name = &field.wire_name;
        if field.attrs.is_id {
            quote! {
                fields.insert(
                    ::std::string::String::from(#wire_name),
                    syncable_state::SnapshotValue::String(snapshot.#field_ident),
                );
            }
        } else {
            let ty = &field.ty;
            quote! {
                fields.insert(
                    ::std::string::String::from(#wire_name),
                    <#ty as syncable_state::SnapshotCodec>::snapshot_to_value(snapshot.#field_ident),
                );
            }
        }
    });

    let from_value_fields = state.fields.iter().map(|field| {
        let field_ident = &field.ident;
        if field.attrs.is_skip {
            quote! {
                #field_ident: ::core::default::Default::default()
            }
        } else if field.attrs.is_id {
            let ty = &field.ty;
            let wire_name = &field.wire_name;
            quote! {
                #field_ident: match fields.remove(#wire_name) {
                    Some(syncable_state::SnapshotValue::String(value)) => <#ty as ::core::str::FromStr>::from_str(&value).map_err(|_| syncable_state::SyncError::InvalidSnapshotValue)?,
                    _ => return Err(syncable_state::SyncError::InvalidSnapshotValue),
                }
            }
        } else {
            let ty = &field.ty;
            let wire_name = &field.wire_name;
            quote! {
                #field_ident: {
                    let mut child_root = root_path.clone().into_vec();
                    child_root.push(syncable_state::PathSegment::Field(::std::string::String::from(#wire_name)));
                    let value = fields.remove(#wire_name).ok_or(syncable_state::SyncError::InvalidSnapshotValue)?;
                    <#ty as syncable_state::SnapshotCodec>::snapshot_from_value(
                        syncable_state::SyncPath::new(child_root),
                        value,
                    )?
                }
            }
        }
    });

    quote! {
        impl #impl_generics syncable_state::SnapshotCodec for #ident #ty_generics #where_clause {
            fn snapshot_to_value(snapshot: Self::Snapshot) -> syncable_state::SnapshotValue {
                let mut fields = ::std::collections::BTreeMap::new();
                #(#to_value_fields)*
                syncable_state::SnapshotValue::Map(fields)
            }

            fn snapshot_from_value(
                root_path: syncable_state::SyncPath,
                value: syncable_state::SnapshotValue,
            ) -> ::core::result::Result<Self, syncable_state::SyncError> {
                let syncable_state::SnapshotValue::Map(mut fields) = value else {
                    return Err(syncable_state::SyncError::InvalidSnapshotValue);
                };

                let value = Self {
                    #(#from_value_fields,)*
                };

                if !fields.is_empty() {
                    return Err(syncable_state::SyncError::InvalidSnapshotValue);
                }

                Ok(value)
            }
        }
    }
}
