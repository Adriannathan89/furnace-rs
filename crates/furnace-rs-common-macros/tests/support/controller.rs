//! Unit tests for managed-controller expansion.

use super::*;
use quote::ToTokens;

fn normalized(tokens: impl ToTokens) -> String {
    tokens
        .to_token_stream()
        .to_string()
        .split_whitespace()
        .collect()
}

#[test]
fn rejects_seventeen_controller_dependencies_with_a_focused_error() {
    let fields = (0..17)
        .map(|i| format!("field{i}: String"))
        .collect::<Vec<_>>()
        .join(",");
    let item = syn::parse_str(&format!("struct TooMany {{ {fields} }}")).unwrap();
    let error =
        expand_controller_with_common(item, &syn::parse_quote!(furnace_rs_common)).unwrap_err();
    assert!(error.to_string().contains("at most sixteen dependencies"));
}

#[test]
fn records_a_managed_controller_and_its_static_seal_callback() {
    let item = syn::parse_str("pub struct Controller;").unwrap();
    let expanded = normalized(
        expand_controller_with_common(item, &syn::parse_quote!(furnace_rs_common)).unwrap(),
    );
    assert!(expanded.contains("ControllerDescriptor::new"));
    assert_eq!(
        expanded.matches("with_namespace(module_path!())").count(),
        2
    );
    assert!(!expanded.contains("RouteContractDescriptor"));
}

#[test]
fn generated_suffix_is_stable_and_sensitive_to_the_struct_shape() {
    let first: ItemStruct = syn::parse_str("struct Controller;").unwrap();
    let second: ItemStruct = syn::parse_str("struct Controller { value: i32 }").unwrap();
    let ident: Ident = syn::parse_str("Controller").unwrap();
    assert_eq!(
        generated_suffix(&first, &ident),
        generated_suffix(&first, &ident)
    );
    assert_ne!(
        generated_suffix(&first, &ident),
        generated_suffix(&second, &ident)
    );
}

#[test]
fn normalizes_self_types_in_nested_fields_and_expressions() {
    let handle: Ident = syn::parse_str("Controller").unwrap();
    let mut ty: Type = syn::parse_str("Option<Self>").unwrap();
    normalize_self_type(&mut ty, &handle);
    assert_eq!(ty.to_token_stream().to_string(), "Option < Controller >");

    let mut expression: ExprPath = syn::parse_str("Self::new").unwrap();
    SelfTypeNormalizer { handle: &handle }.visit_expr_path_mut(&mut expression);
    assert_eq!(
        expression.to_token_stream().to_string(),
        "Controller :: new"
    );
}

#[test]
fn classifies_controller_attributes() {
    let item: ItemStruct = syn::parse_str(
        "#[repr(C)] #[doc = \"docs\"] #[allow(dead_code)] #[derive(Clone)] struct Controller;",
    )
    .unwrap();
    assert!(is_repr(&&item.attrs[0]));
    assert!(is_doc(&&item.attrs[1]));
    assert!(is_supported_attribute(&&item.attrs[1]));
    assert!(is_supported_attribute(&&item.attrs[2]));
    assert!(!is_supported_attribute(&&item.attrs[3]));
}

#[test]
fn rejects_controller_shapes_before_resolving_paths() {
    let cases = [
        quote!(
            struct Controller<T>;
        ),
        quote!(
            struct Controller(i32);
        ),
        quote!(
            #[repr(C)]
            struct Controller;
        ),
        quote!(
            #[derive(Clone)]
            struct Controller;
        ),
        quote!(
            struct Controller {
                #[derive(Clone)]
                value: i32,
            }
        ),
    ];
    for item in cases {
        let item = syn::parse2(item).expect("controller should parse");
        let error = expand_controller_with_common(item, &syn::parse_quote!(furnace_rs_common))
            .expect_err("controller shape must fail");
        assert!(
            error.to_string().contains("controller") || error.to_string().contains("attributes")
        );
    }
}
