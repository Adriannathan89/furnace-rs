//! Public instance and dependency injection coverage for password hashing.
#![cfg(feature = "password-hasher")]

use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{Error, PasswordHash},
};
use furnace_rs_common::{
    DefaultPassowrdHasher, PasswordHasher, PasswordHasherCauldron, PasswordHasherService,
};
use furnace_rs_core::Furnace;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct CountingHasher(Arc<AtomicUsize>);

impl PasswordHasherService for CountingHasher {
    fn hash_password(&self, password: &str) -> Result<String, Error> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Ok(format!("custom:{password}"))
    }

    fn validate_password(&self, password: &str, hashed: &str) -> Result<bool, Error> {
        self.0.fetch_add(1, Ordering::SeqCst);
        if hashed == "malformed" {
            return Err(Error::PhcStringField);
        }
        Ok(hashed == format!("custom:{password}"))
    }
}

#[test]
fn clones_share_a_custom_backend_and_propagate_its_results() {
    let calls = Arc::new(AtomicUsize::new(0));
    let hasher = PasswordHasher::new(CountingHasher(calls.clone()));
    let cloned = hasher.clone();
    assert_eq!(hasher.hash_password("secret").unwrap(), "custom:secret");
    assert_eq!(
        cloned.validate_password("secret", "custom:secret"),
        Ok(true)
    );
    assert_eq!(
        hasher.validate_password("wrong", "custom:secret"),
        Ok(false)
    );
    assert_eq!(
        cloned.validate_password("secret", "malformed"),
        Err(Error::PhcStringField)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 4);
}

#[test]
fn clones_passed_to_different_functions_share_backend_state_and_lifetime() {
    struct StatefulHasher {
        // Keep state inside the backend so separate backend copies cannot share it.
        hashes: AtomicUsize,
        drops: Arc<AtomicUsize>,
    }

    impl PasswordHasherService for StatefulHasher {
        fn hash_password(&self, password: &str) -> Result<String, Error> {
            let count = self.hashes.fetch_add(1, Ordering::SeqCst) + 1;
            Ok(format!("{password}:{count}"))
        }

        fn validate_password(&self, password: &str, hashed: &str) -> Result<bool, Error> {
            let count = self.hashes.load(Ordering::SeqCst);
            Ok(hashed == format!("{password}:{count}"))
        }
    }

    impl Drop for StatefulHasher {
        fn drop(&mut self) {
            self.drops.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn hash_in_another_function(hasher: PasswordHasher) -> String {
        hasher.hash_password("secret").unwrap()
    }

    fn validate_in_another_function(hasher: PasswordHasher, hashed: &str) -> bool {
        hasher.validate_password("secret", hashed).unwrap()
    }

    let drops = Arc::new(AtomicUsize::new(0));
    let hasher = PasswordHasher::new(StatefulHasher {
        hashes: AtomicUsize::new(0),
        drops: drops.clone(),
    });
    let retained = hasher.clone();

    let first = hash_in_another_function(hasher.clone());
    assert_eq!(first, "secret:1");
    assert!(validate_in_another_function(retained.clone(), &first));

    let second = hash_in_another_function(retained.clone());
    assert_eq!(second, "secret:2");
    assert!(!validate_in_another_function(hasher.clone(), &first));
    assert!(validate_in_another_function(hasher.clone(), &second));

    drop(hasher);
    assert_eq!(drops.load(Ordering::SeqCst), 0);
    assert!(validate_in_another_function(retained, &second));
    assert_eq!(drops.load(Ordering::SeqCst), 1);
}

struct FailingHasher;
impl PasswordHasherService for FailingHasher {
    fn hash_password(&self, _: &str) -> Result<String, Error> {
        Err(Error::Password)
    }
    fn validate_password(&self, _: &str, _: &str) -> Result<bool, Error> {
        Err(Error::Password)
    }
}

#[test]
fn custom_backend_errors_are_not_suppressed() {
    let hasher = PasswordHasher::new(FailingHasher);
    assert_eq!(hasher.hash_password("secret"), Err(Error::Password));
    assert_eq!(
        hasher.validate_password("secret", "anything"),
        Err(Error::Password)
    );
}

#[test]
fn configured_backend_encodes_its_settings_and_default_verifies_them() {
    let params = Params::new(32, 1, 1, Some(24)).unwrap();
    let backend =
        DefaultPassowrdHasher::new(Argon2::new(Algorithm::Argon2i, Version::V0x13, params));
    let hasher = PasswordHasher::new(backend);
    let hashed = hasher.hash_password("secret").unwrap();
    let second = hasher.hash_password("secret").unwrap();
    let parsed = PasswordHash::new(&hashed).unwrap();
    assert_eq!(parsed.algorithm.as_str(), "argon2i");
    assert_eq!(parsed.params.get_decimal("m"), Some(32));
    assert_eq!(parsed.params.get_decimal("t"), Some(1));
    assert_eq!(parsed.params.get_decimal("p"), Some(1));
    assert_eq!(parsed.hash.unwrap().len(), 24);
    assert_ne!(hashed, second);
    assert_eq!(hasher.validate_password("secret", &hashed), Ok(true));
    assert_eq!(
        PasswordHasher::default().validate_password("secret", &hashed),
        Ok(true)
    );
    assert_eq!(
        PasswordHasher::default().validate_password("wrong", &hashed),
        Ok(false)
    );
}

#[test]
fn default_instance_preserves_helper_semantics() {
    let hasher = PasswordHasher::default();
    let hashed = hasher.hash_password("密碼🔐").unwrap();
    assert_eq!(
        PasswordHash::new(&hashed).unwrap().algorithm.as_str(),
        "argon2id"
    );
    assert_eq!(hasher.validate_password("密碼🔐", &hashed), Ok(true));
    assert!(hasher.validate_password("secret", "invalid").is_err());
    let unsupported = "$unsupported$v=19$m=19456,t=2,p=1$c29tZXNhbHQ$YWJjZGVmZ2hpamtsbW5vcA";
    assert_eq!(hasher.validate_password("secret", unsupported), Ok(false));
}

mod consumer {
    use furnace_rs_common::PasswordHasher;
    #[furnace_rs_core::burner]
    pub struct Consumer {
        #[inject]
        pub hasher: PasswordHasher,
    }
    #[furnace_rs_core::cauldron(providers = [Consumer], exports = [Consumer])]
    pub struct ConsumerCauldron;
}

#[furnace_rs_core::cauldron(imports = [PasswordHasherCauldron, consumer::ConsumerCauldron])]
struct Root;

#[tokio::test]
async fn global_default_and_supplied_override_reach_other_modules() {
    for custom in [false, true] {
        let calls = Arc::new(AtomicUsize::new(0));
        let mut builder = Furnace::builder();
        builder.root::<Root>().unwrap();
        if custom {
            builder
                .provide(PasswordHasher::new(CountingHasher(calls.clone())))
                .unwrap();
        }
        let application = builder.build().await.unwrap();
        let consumer = application
            .context()
            .resolve::<consumer::Consumer>()
            .unwrap();
        let hashed = consumer.hasher.hash_password("secret").unwrap();
        assert_eq!(
            consumer.hasher.validate_password("secret", &hashed),
            Ok(true)
        );
        assert_eq!(calls.load(Ordering::SeqCst), if custom { 2 } else { 0 });
    }
}
