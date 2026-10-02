use super::*;
use quote::quote;

#[test]
fn test_fn_emits_scoped_cargo_test_and_preserves_return_type() {
    let item: syn::ItemFn = syn::parse2(quote! {
        #[ignore]
        pub async fn example() -> Result<(), ()> { let _ = test_fixture(); Ok(()) }
    })
    .unwrap();
    let expanded = expand_with_path(item, syn::parse_quote!(::renamed_testing));
    let function: syn::ItemFn = syn::parse2(expanded).unwrap();
    assert_eq!(function.sig.ident, "example");
    assert!(matches!(function.sig.output, syn::ReturnType::Type(..)));
    let text = quote!(#function).to_string();
    assert!(text.contains("cfg (test)"));
    assert!(text.contains("tokio :: test"));
    assert!(text.contains("crate ="));
    assert!(text.contains("fn test_fixture"));
    assert!(matches!(
        function.block.stmts[0],
        syn::Stmt::Item(syn::Item::Fn(_))
    ));
    assert!(text.contains("ignore"));
}
