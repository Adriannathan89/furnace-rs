//! Verifies password helper exports with only the password hasher feature enabled.

#![cfg(feature = "password-hasher")]

#[test]
fn password_helpers_are_available_through_public_exports() {
    use furnace_rs::{hash_password, validate_password};

    let password = "public API password";
    let hashed = hash_password(password).expect("hash through facade");
    assert_eq!(validate_password(password, &hashed), Ok(true));
    assert_eq!(
        furnace_rs::prelude::validate_password(password, &hashed),
        Ok(true)
    );
    assert_eq!(
        furnace_rs::common::validate_password(password, &hashed),
        Ok(true)
    );
    // Check the creation helper is exported at each entry point too.
    let _ = furnace_rs::prelude::hash_password;
    let _ = furnace_rs::common::hash_password;
}
