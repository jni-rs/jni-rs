use syn::{Ident, Token};

/// Helper function to resolve the jni crate path using proc_macro_crate
fn resolve_jni_crate() -> syn::Path {
    let Ok(found_crate) = proc_macro_crate::crate_name("jni") else {
        return syn::parse_quote!(::jni);
    };

    match found_crate {
        // Note: if we map `Itself` to `crate` then that won't work with doc tests
        //
        // It's kind of a pain but we instead use `extern crate self as jni;` whenever we use
        // `jni-macros` in the `jni` crate itself.
        //
        // See: <https://github.com/bkchr/proc-macro-crate/issues/11>
        proc_macro_crate::FoundCrate::Itself => syn::parse_quote!(::jni),
        proc_macro_crate::FoundCrate::Name(name) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            syn::parse_quote!( ::#ident )
        }
    }
}

pub fn parse_jni_crate_override(input: &syn::parse::ParseStream) -> Result<syn::Path, syn::Error> {
    let mut jni_path: Option<syn::Path> = None;

    // First check for a special-case `jni = path` property for overriding the jni crate path
    if !input.is_empty() {
        let lookahead = input.lookahead1();
        if lookahead.peek(Ident) {
            // Peek at the first property name
            let fork = input.fork();
            let first_property: Ident = fork.parse()?;

            #[allow(clippy::cmp_owned)]
            if first_property.to_string() == "jni" {
                // Check if it's followed by '='
                if fork.peek(Token![=]) {
                    // Parse the jni property
                    let _property_name: Ident = input.parse()?;
                    input.parse::<Token![=]>()?;
                    jni_path = Some(input.parse()?);

                    // Skip comma after jni property
                    if input.peek(Token![,]) {
                        input.parse::<Token![,]>()?;
                    }
                }
            }
        }
    }

    Ok(jni_path.unwrap_or_else(resolve_jni_crate))
}
