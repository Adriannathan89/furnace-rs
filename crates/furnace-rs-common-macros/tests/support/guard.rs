//! Unit tests for Passport guard parsing and inheritance.

use super::*;

fn parsed(source: &str) -> GuardSpec {
    syn::parse_str(source).unwrap_or_else(|error| panic!("{source}: {error}"))
}

#[cfg(feature = "cookies")]
#[test]
fn parses_the_complete_guard_grammar() {
    let spec = parsed(
        r#"strategy = "jwt", principal = ClaimsPrincipal<UserClaims>, source = cookie("access"), roles(any = ["user"]), permissions(all = ["profile:read"]), predicates = [can_read, policy::owns]"#,
    );

    assert_eq!(spec.strategy.unwrap().value(), "jwt");
    assert!(matches!(spec.source, Some(TokenSourceSpec::Cookie(_))));
    assert_eq!(spec.roles.unwrap().values.len(), 1);
    assert_eq!(spec.permissions.unwrap().values.len(), 1);
    assert_eq!(spec.predicates.unwrap().len(), 2);
}

#[cfg(not(feature = "cookies"))]
#[test]
fn cookie_sources_require_the_cookie_capability() {
    let error = match syn::parse_str::<GuardSpec>(
        r#"strategy = "jwt", principal = UserPrincipal, source = cookie("access")"#,
    ) {
        Err(error) => error,
        Ok(_) => panic!("a cookie source must require the cookie capability"),
    };

    assert_eq!(
        error.to_string(),
        "cookie token sources require the `cookies` feature"
    );
}

#[test]
fn rejects_duplicate_and_mixed_skip_fields() {
    for source in [
        r#"strategy = "jwt", strategy = "other""#,
        r#"predicate = first, predicates = [second]"#,
        r#"skip, strategy = "jwt""#,
        r#"roles(any = [] )"#,
    ] {
        assert!(syn::parse_str::<GuardSpec>(source).is_err(), "{source}");
    }
}

#[test]
fn rejects_invalid_names_sources_and_policy_forms() {
    for source in [
        r#"strategy = "JWT", principal = UserPrincipal"#,
        r#"strategy = "jwt", principal = UserPrincipal, source = header"#,
        r#"strategy = "jwt", principal = UserPrincipal, roles(one = ["user"])"#,
        r#"strategy = "jwt", principal = UserPrincipal, unknown = "value""#,
    ] {
        assert!(syn::parse_str::<GuardSpec>(source).is_err(), "{source}");
    }

    #[cfg(feature = "cookies")]
    assert!(
        syn::parse_str::<GuardSpec>(
            r#"strategy = "jwt", principal = UserPrincipal, source = cookie("bad;name")"#,
        )
        .is_err()
    );
}

#[test]
fn policies_require_strategy_and_principal() {
    for source in [r#"strategy = "jwt""#, "principal = UserPrincipal"] {
        assert!(complete_policy(&parsed(source)).is_err());
    }
}
#[test]
fn static_policy_metadata_preserves_all_rules_without_inventory_activation() {
    let effective = complete_policy(&parsed(r#"strategy = "jwt", principal = UserPrincipal, roles(any = ["user"]), permissions(all = ["read"]), predicate = owns"#)).unwrap();
    assert_eq!(effective.roles.as_ref().unwrap().values[0].value(), "user");
    assert_eq!(
        effective.permissions.as_ref().unwrap().values[0].value(),
        "read"
    );
    assert_eq!(effective.predicates.len(), 1);
    let (_, tokens) = effective.static_tokens_with_registration(
        &syn::parse_quote!(furnace_rs_common),
        &syn::parse_quote!(Policy),
        &syn::parse_quote!(seal),
        &[],
    );
    assert!(!tokens.to_string().contains("inventory"));
    assert!(tokens.to_string().contains("with_namespace"));
}
