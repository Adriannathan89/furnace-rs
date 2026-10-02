#[cfg(test)]
    mod tests {
        use super::*;
        use quote::ToTokens;

        #[test]
        fn rejects_seventeen_managed_dependencies_with_a_focused_error() {
            let fields = (0..17).map(|i| format!("field{i}: String")).collect::<Vec<_>>().join(",");
            let item: TokenStream = format!("struct TooMany {{ {fields} }}").parse().unwrap();
            let error = expand(ManagedKind::Service, TokenStream::new(), item).unwrap_err();
            assert!(error.to_string().contains("at most sixteen dependencies"));
        }

        fn kind_name(kind: &ManagedKind) -> &'static str {
            kind.attribute_name()
        }

        #[test]
        fn managed_kind_metadata_is_distinct() {
            assert_eq!(kind_name(&ManagedKind::Service), "burner");
            assert_eq!(kind_name(&ManagedKind::Repository), "storage");
            assert_eq!(
                ManagedKind::Service
                    .provider_kind(&syn::parse_quote!(furnace_rs))
                    .to_string(),
                "furnace_rs :: ProviderKind :: Service"
            );
            assert_eq!(
                ManagedKind::Repository
                    .provider_kind(&syn::parse_quote!(furnace_rs))
                    .to_string(),
                "furnace_rs :: ProviderKind :: Repository"
            );
            assert!(ManagedKind::Service.supported_form().contains("burner"));
            assert!(
                ManagedKind::Repository
                    .supported_form()
                    .contains("storage")
            );
        }

        #[test]
        fn managed_expansions_record_the_declaration_namespace() {
            for kind in [ManagedKind::Service, ManagedKind::Repository] {
                let item: ItemStruct = syn::parse_quote! {
                    pub struct Managed;
                };
                let expanded =
                    expand_managed_with_core(kind, item, syn::parse_quote!(furnace_rs_core))
                        .expect("managed provider should expand")
                        .to_string();

                assert!(
                    expanded.contains(". with_namespace (module_path ! ())"),
                    "expanded descriptor did not record its namespace: {expanded}"
                );
            }
        }

        #[test]
        fn rejects_managed_provider_shapes_before_path_resolution() {
            for kind in [ManagedKind::Service, ManagedKind::Repository] {
                let cases = [
                    (
                        quote::quote!(unexpected),
                        quote::quote!(
                            struct Managed;
                        ),
                    ),
                    (
                        quote::quote!(),
                        quote::quote!(
                            struct Managed(i32);
                        ),
                    ),
                    (
                        quote::quote!(),
                        quote::quote!(
                            struct Managed<T>;
                        ),
                    ),
                    (
                        quote::quote!(),
                        quote::quote!(
                            #[repr(C)]
                            struct Managed;
                        ),
                    ),
                    (
                        quote::quote!(),
                        quote::quote!(
                            #[derive(Clone)]
                            struct Managed;
                        ),
                    ),
                    (
                        quote::quote!(),
                        quote::quote!(
                            struct Managed {
                                #[allow(dead_code)]
                                value: i32,
                            }
                        ),
                    ),
                ];
                for (arguments, item) in cases {
                    let error = expand(kind_ref(&kind), arguments, item)
                        .expect_err("managed provider shape must fail");
                    assert!(
                        error.to_string().contains("supports only")
                            || error.to_string().contains("attributes")
                    );
                }
            }
        }

        fn kind_ref(kind: &ManagedKind) -> ManagedKind {
            match kind {
                ManagedKind::Service => ManagedKind::Service,
                ManagedKind::Repository => ManagedKind::Repository,
            }
        }

        #[test]
        fn normalizes_self_types_inside_managed_fields() {
            let handle: Ident = syn::parse_str("Managed").unwrap();
            let mut ty: Type = syn::parse_str("Option<Self>").unwrap();
            normalize_self_type(&mut ty, &handle);
            assert_eq!(ty.to_token_stream().to_string(), "Option < Managed >");

            let mut expression: ExprPath = syn::parse_str("Self::value").unwrap();
            SelfTypeNormalizer { handle: &handle }.visit_expr_path_mut(&mut expression);
            assert_eq!(expression.to_token_stream().to_string(), "Managed :: value");
        }
    }
