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

#[test]
fn password_hasher_instances_are_available_through_public_exports() {
    use furnace_rs::{
        DefaultPassowrdHasher, PasswordHasher, PasswordHasherCauldron, PasswordHasherError,
        PasswordHasherResult, PasswordHasherService,
    };

    fn assert_service<T: PasswordHasherService>() {}
    assert_service::<DefaultPassowrdHasher>();
    let _module = PasswordHasherCauldron;
    let hasher = PasswordHasher::default();
    let hashed: PasswordHasherResult<String> = hasher.hash_password("facade instance");
    let hashed = hashed.unwrap();
    assert_eq!(
        hasher.validate_password("facade instance", &hashed),
        Ok(true)
    );
    let _: Option<PasswordHasherError> = None;
    let _: furnace_rs::prelude::PasswordHasher = hasher.clone();
    let _: furnace_rs::common::PasswordHasher = hasher.clone();
    let _: furnace_rs::password_hasher::PasswordHasher = hasher;
    let _ = furnace_rs::prelude::DefaultPassowrdHasher::default();
    let _ = furnace_rs::prelude::PasswordHasherCauldron;
    let _: furnace_rs::prelude::PasswordHasherResult<()> = Ok(());
    let _: Option<furnace_rs::prelude::PasswordHasherError> = None;
    fn assert_prelude_service<T: furnace_rs::prelude::PasswordHasherService>() {}
    assert_prelude_service::<DefaultPassowrdHasher>();
}
