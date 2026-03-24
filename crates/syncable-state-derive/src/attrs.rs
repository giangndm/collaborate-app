use syn::spanned::Spanned;

#[derive(Clone, Default)]
pub struct SyncAttrs {
    pub is_id: bool,
    pub is_skip: bool,
    pub rename: Option<String>,
    pub with: Option<syn::Path>,
}

impl SyncAttrs {
    pub fn wire_name(&self, rust_name: &str) -> String {
        self.rename.clone().unwrap_or_else(|| rust_name.to_owned())
    }
}

pub fn parse_sync_attrs(attrs: &[syn::Attribute]) -> syn::Result<SyncAttrs> {
    let mut parsed = SyncAttrs::default();

    for attr in attrs {
        if !attr.path().is_ident("sync") {
            continue;
        }

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("id") {
                set_flag(&mut parsed.is_id, meta.path.span(), "id")?;
                return Ok(());
            }

            if meta.path.is_ident("skip") {
                set_flag(&mut parsed.is_skip, meta.path.span(), "skip")?;
                return Ok(());
            }

            if meta.path.is_ident("rename") {
                if parsed.rename.is_some() {
                    return Err(syn::Error::new(
                        meta.path.span(),
                        "duplicate `rename` attribute",
                    ));
                }
                let value = meta.value()?;
                let rename = value.parse::<syn::LitStr>()?;
                parsed.rename = Some(rename.value());
                return Ok(());
            }

            if meta.path.is_ident("with") {
                if parsed.with.is_some() {
                    return Err(syn::Error::new(
                        meta.path.span(),
                        "duplicate `with` attribute",
                    ));
                }
                let value = meta.value()?;
                parsed.with = Some(value.parse::<syn::Path>()?);
                return Ok(());
            }

            Err(meta.error("unsupported `sync` attribute"))
        })?;
    }

    Ok(parsed)
}

fn set_flag(slot: &mut bool, span: proc_macro2::Span, name: &str) -> syn::Result<()> {
    if *slot {
        return Err(syn::Error::new(
            span,
            format!("duplicate `{name}` attribute"),
        ));
    }

    *slot = true;
    Ok(())
}
