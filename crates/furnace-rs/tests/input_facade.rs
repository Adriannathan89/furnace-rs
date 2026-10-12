//! Input validation is available through the HTTP facade and prelude.
#![cfg(feature = "http")]

use furnace_rs::prelude::*;

#[derive(furnace_rs::Input)]
struct RootInput {
    #[validate(email)]
    email: String,
}

#[derive(Input)]
struct PreludeInput {
    #[validate(nested)]
    inner: RootInput,
}

#[test]
fn facade_and_prelude_expose_trait_and_derive() {
    let input = PreludeInput {
        inner: RootInput {
            email: "invalid".into(),
        },
    };
    let errors: furnace_rs::ValidationErrors = input.validate().unwrap_err();
    assert_eq!(
        errors.issues()[0].path(),
        [
            furnace_rs::ValidationPathSegment::Field("inner".into()),
            furnace_rs::ValidationPathSegment::Field("email".into())
        ]
    );
    let _: ValidationResult = Err(ValidationErrors::from_issue(ValidationIssue::custom(
        "app",
        "application issue",
    )));
}
