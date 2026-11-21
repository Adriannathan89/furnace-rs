//! In-process tests: no listener, no `.env`, and a cheap password hasher.
use furnace_rs::prelude::*;
use furnace_rs_testing::http_types::StatusCode;
use serde_json::json;

use super::{
    controller::AccountController, repository::AccountRepository, service::AccountService,
};

const PASSWORD: &str = "correct horse battery staple";

/// Supplies replace the providers a real run would construct from `.env`.
fn jwt() -> JwtService {
    let config = ConfigBuilder::new()
        .source(furnace_rs::core::MapSource::new(
            "test",
            [("passport.secret", "test-only-signing-key-with-32-bytes!")],
        ))
        .build()
        .unwrap();

    JwtService::from_config(&config).unwrap()
}

// `AccountRepository` implements `Injector` by hand, so the fixture cannot
// discover it and each test supplies one; macro-generated providers such as
// `AccountService` are found automatically.

/// Minimal Argon2 costs keep the suite fast; production uses the defaults.
fn fast_hasher() -> PasswordHasher {
    PasswordHasher::new(DefaultPasswordHasher::with_costs(32, 1, 1).unwrap())
}

#[furnace_rs::test]
async fn stores_a_hash_and_authenticates_only_the_right_password() {
    test_fixture()
        .provide(jwt())
        .provide(fast_hasher())
        .provide(AccountRepository::default())
        .subject::<AccountService>()
        .run(|context| async move {
            let accounts = context.resolve::<AccountService>().unwrap();

            let created = accounts.register("ada", PASSWORD).await.unwrap().unwrap();
            assert!(created.password_hash.starts_with("$argon2id$"));
            assert!(accounts.register("ada", PASSWORD).await.unwrap().is_none());

            let known = accounts.authenticate("ada", PASSWORD).await.unwrap();
            assert_eq!(known.map(|account| account.id), Some(created.id));
            assert!(
                accounts
                    .authenticate("ada", "wrong password")
                    .await
                    .unwrap()
                    .is_none()
            );
            assert!(
                accounts
                    .authenticate("nobody", PASSWORD)
                    .await
                    .unwrap()
                    .is_none()
            );
        })
        .await
        .unwrap();
}

#[furnace_rs::test]
async fn http_boundary_validates_input_and_hides_which_part_failed() {
    test_fixture()
        .provide(jwt())
        .provide(fast_hasher())
        .provide(AccountRepository::default())
        .controller::<AccountController>()
        .run(|client| async move {
            let credentials = json!({ "username": "ada", "password": PASSWORD });

            client
                .post("/accounts/register")
                .json(&json!({ "username": "ada", "password": "short" }))
                .unwrap()
                .send()
                .await
                .unwrap()
                .assert_status(StatusCode::UNPROCESSABLE_ENTITY);

            let register = || {
                client
                    .post("/accounts/register")
                    .json(&credentials)
                    .unwrap()
            };
            register()
                .send()
                .await
                .unwrap()
                .assert_status(StatusCode::OK)
                .assert_json(json!({ "id": 1, "username": "ada" }));
            register()
                .send()
                .await
                .unwrap()
                .assert_status(StatusCode::CONFLICT);

            client
                .post("/accounts/login")
                .json(&credentials)
                .unwrap()
                .send()
                .await
                .unwrap()
                .assert_status(StatusCode::OK);

            let rejected = json!({
                "error": { "code": "unauthorized", "message": "invalid credentials" }
            });
            for attempt in [
                json!({ "username": "ada", "password": "wrong password!!" }),
                json!({ "username": "nobody", "password": PASSWORD }),
            ] {
                client
                    .post("/accounts/login")
                    .json(&attempt)
                    .unwrap()
                    .send()
                    .await
                    .unwrap()
                    .assert_status(StatusCode::UNAUTHORIZED)
                    .assert_json(rejected.clone());
            }
        })
        .await
        .unwrap();
}
