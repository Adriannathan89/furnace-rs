//! Parsing of declarative registration options.
use quote::ToTokens;
use std::collections::HashSet;
use syn::{
    Error, Ident, Path, Token, Type, bracketed,
    parse::{Parse, ParseStream},
};

pub(super) struct Provider {
    pub output: Type,
    pub injector: Option<Type>,
}

impl Parse for Provider {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let output = input.parse()?;
        let injector = if input.peek(Token![=>]) {
            input.parse::<Token![=>]>()?;
            Some(input.parse()?)
        } else {
            None
        };
        Ok(Self { output, injector })
    }
}

#[derive(Default)]
pub(super) struct Declaration {
    pub providers: Vec<Provider>,
    pub controllers: Vec<Type>,
    pub imports: Vec<Path>,
    pub exports: Vec<Type>,
    pub global: bool,
    pub root: bool,
}

fn unique<T: ToTokens>(items: &[T]) -> syn::Result<()> {
    let mut seen = HashSet::new();
    for item in items {
        if !seen.insert(item.to_token_stream().to_string()) {
            return Err(Error::new_spanned(item, "duplicate declaration entry"));
        }
    }
    Ok(())
}

impl Parse for Declaration {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut declaration = Self::default();
        let mut seen = HashSet::new();
        while !input.is_empty() {
            let name: Ident = input.parse()?;
            let option = name.to_string();
            if !seen.insert(option.clone()) {
                return Err(Error::new_spanned(name, "duplicate cauldron option"));
            }
            match option.as_str() {
                "root" => declaration.root = true,
                "global" => declaration.global = true,
                "providers" | "controllers" | "imports" | "exports" => {
                    input.parse::<Token![=]>()?;
                    let list;
                    bracketed!(list in input);
                    match option.as_str() {
                        "providers" => {
                            declaration.providers = list
                                .parse_terminated(Provider::parse, Token![,])?
                                .into_iter()
                                .collect();
                            unique(
                                &declaration
                                    .providers
                                    .iter()
                                    .map(|p| &p.output)
                                    .collect::<Vec<_>>(),
                            )?;
                        }
                        "controllers" => {
                            declaration.controllers = list
                                .parse_terminated(Type::parse, Token![,])?
                                .into_iter()
                                .collect();
                            unique(&declaration.controllers)?;
                        }
                        "imports" => {
                            declaration.imports = list
                                .parse_terminated(Path::parse, Token![,])?
                                .into_iter()
                                .collect();
                            unique(&declaration.imports)?;
                        }
                        "exports" => {
                            declaration.exports = list
                                .parse_terminated(Type::parse, Token![,])?
                                .into_iter()
                                .collect();
                            unique(&declaration.exports)?;
                        }
                        _ => unreachable!(),
                    }
                }
                _ => {
                    return Err(Error::new_spanned(
                        name,
                        "unknown cauldron option; expected providers, controllers, imports, exports, global, or root",
                    ));
                }
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(declaration)
    }
}

#[cfg(test)]
mod tests {
    use super::Declaration;
    use quote::quote;

    #[test]
    fn rejects_duplicate_options_entries_and_malformed_bindings() {
        for tokens in [
            quote!(root, root),
            quote!(global = true),
            quote!(other),
            quote!(providers = [A, A]),
            quote!(providers=[A => B, A => C]),
            quote!(controllers = [A, A]),
            quote!(imports = [a::B, a::B]),
            quote!(exports = [A, A]),
            quote!(providers=[A =>]),
            quote!(providers = A),
        ] {
            assert!(
                syn::parse2::<Declaration>(tokens.clone()).is_err(),
                "{tokens}"
            );
        }
    }

    #[test]
    fn parses_qualified_native_bindings_flags_and_empty_lists() {
        let declaration: Declaration = syn::parse2(quote!(root, global, providers=[std::sync::Arc<String> => native::Factory, Service], imports=[infra::Database], controllers=[], exports=[std::sync::Arc<String>],)).unwrap();
        assert!(declaration.root && declaration.global);
        assert_eq!(declaration.providers.len(), 2);
        assert!(declaration.providers[0].injector.is_some());
        assert!(declaration.controllers.is_empty());
    }
}
