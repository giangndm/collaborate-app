use syn::{Data, DeriveInput, Fields, Ident, Type, Visibility};

use crate::attrs::{SyncAttrs, parse_sync_attrs};

#[derive(Clone)]
pub struct ParsedState {
    pub ident: Ident,
    pub vis: Visibility,
    pub generics: syn::Generics,
    pub fields: Vec<ParsedField>,
}

#[derive(Clone)]
pub struct ParsedField {
    pub ident: Ident,
    pub ty: Type,
    pub wire_name: String,
    pub attrs: SyncAttrs,
}

impl ParsedField {
    pub fn is_included(&self) -> bool {
        !self.attrs.is_skip
    }

    pub fn is_routable(&self) -> bool {
        self.is_included() && !self.attrs.is_id
    }
}

pub fn parse_state(input: DeriveInput) -> syn::Result<ParsedState> {
    let ident = input.ident;
    let vis = input.vis;
    let generics = input.generics;

    let Data::Struct(item) = input.data else {
        return Err(syn::Error::new_spanned(
            ident,
            "SyncableState can only be derived for structs",
        ));
    };

    let Fields::Named(named) = item.fields else {
        return Err(syn::Error::new_spanned(
            ident,
            "SyncableState only supports named-field structs",
        ));
    };

    let mut fields = Vec::with_capacity(named.named.len());
    for field in named.named {
        let ident = field.ident.expect("named fields always have idents");
        let attrs = parse_sync_attrs(&field.attrs)?;
        let wire_name = attrs.wire_name(&ident.to_string());
        fields.push(ParsedField {
            ident,
            ty: field.ty,
            wire_name,
            attrs,
        });
    }

    Ok(ParsedState {
        ident,
        vis,
        generics,
        fields,
    })
}
