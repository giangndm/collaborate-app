use quote::quote;
use syn::parse_quote;

use crate::{parse::ParsedState, schema, snapshot};

pub fn expand(input: syn::DeriveInput) -> syn::Result<proc_macro2::TokenStream> {
    let state = crate::parse::parse_state(input)?;
    crate::validate::validate_state(&state)?;

    let vec_identity_assertions = expand_vec_identity_assertions(&state);
    let snapshot_def = snapshot::expand_snapshot(&state);
    let syncable_impl = expand_syncable_state_impl(&state);
    let apply_impl = expand_apply_impl(&state);
    let child_apply_impl = expand_apply_child_impl(&state);
    let stable_id_impl = expand_stable_id_impl(&state);
    let snapshot_codec_impl = snapshot::expand_snapshot_codec(&state);

    Ok(quote! {
        #(#vec_identity_assertions)*
        #snapshot_def
        #syncable_impl
        #apply_impl
        #child_apply_impl
        #stable_id_impl
        #snapshot_codec_impl
    })
}

fn expand_vec_identity_assertions(state: &ParsedState) -> Vec<proc_macro2::TokenStream> {
    state
        .fields
        .iter()
        .filter_map(|field| {
            let element_ty = syncable_vec_element_ty(&field.ty)?;
            let assert_struct = syn::Ident::new(
                &format!(
                    "SyncableStateField{}VecElementsMustHaveSyncIdStableIdContract",
                    to_pascal_case(&field.ident.to_string())
                ),
                field.ident.span(),
            );

            Some(quote! {
                const _: () = {
                    struct #assert_struct<T: syncable_state::StableId>(::core::marker::PhantomData<T>);

                    let _ = ::core::marker::PhantomData::<#assert_struct<#element_ty>>;
                };
            })
        })
        .collect()
}

fn to_pascal_case(name: &str) -> String {
    let mut out = String::new();
    let mut uppercase_next = true;

    for ch in name.chars() {
        if ch == '_' {
            uppercase_next = true;
            continue;
        }

        if uppercase_next {
            out.extend(ch.to_uppercase());
            uppercase_next = false;
        } else {
            out.push(ch);
        }
    }

    out
}

fn syncable_vec_element_ty(ty: &syn::Type) -> Option<&syn::Type> {
    let syn::Type::Path(type_path) = ty else {
        return None;
    };
    let segment = type_path.path.segments.last()?;
    if segment.ident != "SyncableVec" {
        return None;
    }

    let syn::PathArguments::AngleBracketed(args) = &segment.arguments else {
        return None;
    };

    let first = args.args.first()?;
    let syn::GenericArgument::Type(inner) = first else {
        return None;
    };

    Some(inner)
}

pub fn snapshot_generics(state: &ParsedState) -> syn::Generics {
    let mut generics = state.generics.clone();
    let where_clause = generics.make_where_clause();

    for field in state
        .fields
        .iter()
        .filter(|field| field.is_included() && !field.attrs.is_id)
    {
        let ty = &field.ty;
        where_clause
            .predicates
            .push(parse_quote!(#ty: syncable_state::SyncableState));
    }

    generics
}

pub fn impl_generics(state: &ParsedState) -> syn::Generics {
    let mut generics = snapshot_generics(state);
    let where_clause = generics.make_where_clause();

    for field in state.fields.iter().filter(|field| field.is_routable()) {
        let ty = &field.ty;
        where_clause
            .predicates
            .push(parse_quote!(#ty: syncable_state::ApplyChildPath));
        where_clause
            .predicates
            .push(parse_quote!(#ty: syncable_state::SnapshotCodec));
    }

    for field in state.fields.iter().filter(|field| field.attrs.is_skip) {
        let ty = &field.ty;
        where_clause
            .predicates
            .push(parse_quote!(#ty: ::core::default::Default));
    }

    generics
}

fn expand_syncable_state_impl(state: &ParsedState) -> proc_macro2::TokenStream {
    let ident = &state.ident;
    let snapshot_ident = snapshot::snapshot_ident(state);
    let generics = impl_generics(state);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let schema_expr = schema::expand_schema_expr(state);
    let snapshot_expr = snapshot::expand_snapshot_expr(state);
    let rebind_fields = rebind_fields(state);

    quote! {
        impl #impl_generics syncable_state::SyncableState for #ident #ty_generics #where_clause {
            type Snapshot = #snapshot_ident #ty_generics;

            fn snapshot(&self) -> Self::Snapshot {
                #snapshot_expr
            }

            fn should_rebind_root() -> bool
            where
                Self: Sized,
            {
                true
            }

            fn rebind_paths(&mut self, root_path: syncable_state::SyncPath) {
                #(#rebind_fields)*
            }

            fn schema() -> syncable_state::StateSchema {
                #schema_expr
            }
        }
    }
}

fn rebind_fields(state: &ParsedState) -> Vec<proc_macro2::TokenStream> {
    state
        .fields
        .iter()
        .filter(|field| field.is_included() && !field.attrs.is_id)
        .map(|field| {
            let field_ident = &field.ident;
            let wire_name = &field.wire_name;
            quote! {
                {
                    let mut child_root = root_path.clone().into_vec();
                    child_root.push(syncable_state::PathSegment::Field(::std::string::String::from(#wire_name)));
                    syncable_state::SyncableState::rebind_paths(
                        &mut self.#field_ident,
                        syncable_state::SyncPath::new(child_root),
                    );
                }
            }
        })
        .collect()
}

fn expand_apply_impl(state: &ParsedState) -> proc_macro2::TokenStream {
    let ident = &state.ident;
    let generics = impl_generics(state);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();
    let routing = routing_arms(state);

    quote! {
        impl #impl_generics syncable_state::ApplyPath for #ident #ty_generics #where_clause {
            fn apply_path(
                &mut self,
                path: &[syncable_state::PathSegment],
                op: &syncable_state::ChangeOp,
            ) -> ::core::result::Result<(), syncable_state::SyncError> {
                match path {
                    #(#routing)*
                    _ => Err(syncable_state::SyncError::InvalidPath),
                }
            }
        }
    }
}

fn expand_apply_child_impl(state: &ParsedState) -> proc_macro2::TokenStream {
    let ident = &state.ident;
    let generics = impl_generics(state);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    quote! {
        impl #impl_generics syncable_state::ApplyChildPath for #ident #ty_generics #where_clause {
            fn apply_child_path(
                &mut self,
                path: &[syncable_state::PathSegment],
                op: &syncable_state::ChangeOp,
            ) -> ::core::result::Result<(), syncable_state::SyncError> {
                syncable_state::ApplyPath::apply_path(self, path, op)
            }
        }
    }
}

fn expand_stable_id_impl(state: &ParsedState) -> proc_macro2::TokenStream {
    let Some(id_field) = state.fields.iter().find(|field| field.attrs.is_id) else {
        return quote! {};
    };

    let ident = &state.ident;
    let field_ident = &id_field.ident;
    let generics = snapshot_generics(state);
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let ty = &id_field.ty;
    quote! {
        impl #impl_generics syncable_state::StableId for #ident #ty_generics #where_clause {
            type Id = #ty;
            fn stable_id(&self) -> &Self::Id {
                &self.#field_ident
            }
        }
    }
}

fn routing_arms(state: &ParsedState) -> Vec<proc_macro2::TokenStream> {
    state
        .fields
        .iter()
        .filter(|field| field.is_routable())
        .map(|field| {
            let field_ident = &field.ident;
            let wire_name = &field.wire_name;
            quote! {
                [syncable_state::PathSegment::Field(field), tail @ ..] if field == #wire_name => {
                    syncable_state::ApplyChildPath::apply_child_path(&mut self.#field_ident, tail, op)
                }
            }
        })
        .collect()
}
