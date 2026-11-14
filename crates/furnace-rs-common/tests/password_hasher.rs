//! Public instance and dependency injection coverage for password hashing.
#![cfg(feature = "password-hasher")]

use argon2::{
    Algorithm, Argon2, Params, Version,
    password_hash::{Error, PasswordHash},
};
use furnace_rs_common::{
    DefaultPasswordHasher, PasswordHasher, PasswordHasherCauldron, PasswordHasherService,
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
        DefaultPasswordHasher::new(Argon2::new(Algorithm::Argon2i, Version::V0x13, params));
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
fn verification_cost_limits_cover_configured_settings_and_reject_heavier_hashes() {
    use argon2::password_hash::errors::InvalidValue;

    // A time cost just above the default ceiling, kept cheap by minimal memory.
    let heavy_t_cost = DefaultPasswordHasher::DEFAULT_MAX_VERIFY_T_COST + 1;
    let params = Params::new(32, heavy_t_cost, 1, None).unwrap();
    let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, params);
    let too_costly = Err(Error::ParamValueInvalid(InvalidValue::TooLong));

    let configured = DefaultPasswordHasher::new(argon2.clone());
    let hashed = configured.hash_password("secret").unwrap();
    assert_eq!(configured.validate_password("secret", &hashed), Ok(true));

    // The default backend refuses the heavier hash for right and wrong passwords.
    let default = DefaultPasswordHasher::default();
    assert_eq!(default.validate_password("secret", &hashed), too_costly);
    assert_eq!(default.validate_password("wrong", &hashed), too_costly);

    let raised = DefaultPasswordHasher::default().with_verification_limits(
        DefaultPasswordHasher::DEFAULT_MAX_VERIFY_M_COST,
        heavy_t_cost,
        DefaultPasswordHasher::DEFAULT_MAX_VERIFY_P_COST,
    );
    assert_eq!(raised.validate_password("secret", &hashed), Ok(true));
    assert_eq!(raised.validate_password("wrong", &hashed), Ok(false));

    let lowered = DefaultPasswordHasher::new(argon2).with_verification_limits(32, 1, 1);
    assert_eq!(lowered.validate_password("secret", &hashed), too_costly);
}

#[test]
fn optional_validation_spends_one_verification_on_unknown_accounts() {
    let calls = Arc::new(AtomicUsize::new(0));
    let hasher = PasswordHasher::new(CountingHasher(calls.clone()));

    // A stored hash behaves exactly like `validate_password`.
    assert_eq!(
        hasher.validate_optional_password("secret", Some("custom:secret")),
        Ok(true)
    );
    assert_eq!(
        hasher.validate_optional_password("wrong", Some("custom:secret")),
        Ok(false)
    );
    assert_eq!(
        hasher.validate_optional_password("secret", Some("malformed")),
        Err(Error::PhcStringField)
    );
    assert_eq!(calls.load(Ordering::SeqCst), 3);

    // An unknown account costs one decoy hash, once, plus one verification.
    assert_eq!(hasher.validate_optional_password("secret", None), Ok(false));
    assert_eq!(calls.load(Ordering::SeqCst), 5);

    // Clones reuse the decoy, and no password can match it.
    let cloned = hasher.clone();
    assert_eq!(cloned.validate_optional_password("", None), Ok(false));
    assert_eq!(calls.load(Ordering::SeqCst), 6);

    let failing = PasswordHasher::new(FailingHasher);
    assert_eq!(
        failing.validate_optional_password("secret", None),
        Err(Error::Password)
    );
}

#[test]
fn default_backend_rejects_every_password_for_an_unknown_account() {
    let hasher = PasswordHasher::new(DefaultPasswordHasher::with_costs(32, 1, 1).unwrap());

    for password in ["", "secret", "furnace-rs"] {
        assert_eq!(hasher.validate_optional_password(password, None), Ok(false));
    }
}

#[test]
fn cost_constructor_and_reexported_argon2_need_no_extra_dependency() {
    use furnace_rs_common::password_hasher::argon2 as reexported;

    let hashed = DefaultPasswordHasher::with_costs(32, 3, 1)
        .unwrap()
        .hash_password("secret")
        .unwrap();
    let parsed = PasswordHash::new(&hashed).unwrap();
    assert_eq!(parsed.algorithm.as_str(), "argon2id");
    assert_eq!(parsed.version, Some(19));
    assert_eq!(parsed.params.get_decimal("m"), Some(32));
    assert_eq!(parsed.params.get_decimal("t"), Some(3));
    assert_eq!(parsed.params.get_decimal("p"), Some(1));
    assert_eq!(
        PasswordHasher::default().validate_password("secret", &hashed),
        Ok(true)
    );

    // Costs Argon2 cannot run with are reported instead of panicking later.
    assert!(DefaultPasswordHasher::with_costs(1, 1, 1).is_err());
    assert!(DefaultPasswordHasher::with_costs(32, 0, 1).is_err());

    let params = reexported::Params::new(32, 1, 1, None).unwrap();
    let _backend = DefaultPasswordHasher::new(reexported::Argon2::from(params));
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

#[cfg(any(feature = "password-hasher-async", feature = "transport-runtime"))]
mod non_blocking {
    use super::*;
    use std::{sync::mpsc, time::Duration};

    #[tokio::test]
    async fn async_methods_match_their_synchronous_counterparts() {
        let hasher = PasswordHasher::new(DefaultPasswordHasher::with_costs(32, 1, 1).unwrap());
        let hashed = hasher.hash_password_async("secret").await.unwrap();

        assert_eq!(hasher.validate_password("secret", &hashed), Ok(true));
        assert_eq!(
            hasher.validate_password_async("secret", &hashed).await,
            Ok(true)
        );
        assert_eq!(
            hasher.validate_password_async("wrong", &hashed).await,
            Ok(false)
        );
        assert!(
            hasher
                .validate_password_async("secret", "invalid")
                .await
                .is_err()
        );
        assert_eq!(
            hasher
                .validate_optional_password_async("secret", Some(&hashed))
                .await,
            Ok(true)
        );
        assert_eq!(
            hasher
                .validate_optional_password_async("secret", None)
                .await,
            Ok(false)
        );
        assert_eq!(
            PasswordHasher::new(FailingHasher)
                .hash_password_async("secret")
                .await,
            Err(Error::Password)
        );
    }

    /// Hashing completes only after another task on the runtime has run.
    struct WaitsForRuntime(std::sync::Mutex<mpsc::Receiver<()>>);

    impl PasswordHasherService for WaitsForRuntime {
        fn hash_password(&self, _: &str) -> Result<String, Error> {
            self.0
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(10))
                .map_err(|_| Error::Crypto)?;
            Ok("released".to_owned())
        }

        fn validate_password(&self, _: &str, _: &str) -> Result<bool, Error> {
            Ok(false)
        }
    }

    // A single-threaded runtime makes inline hashing starve the releasing task.
    #[tokio::test(flavor = "current_thread")]
    async fn hashing_leaves_the_runtime_thread_free_for_other_tasks() {
        let (release, wait) = mpsc::channel();
        let hasher = PasswordHasher::new(WaitsForRuntime(std::sync::Mutex::new(wait)));

        let (hashed, ()) = tokio::join!(hasher.hash_password_async("secret"), async move {
            tokio::task::yield_now().await;
            release.send(()).unwrap();
        });

        assert_eq!(hashed.unwrap(), "released");
    }

    struct TracksConcurrency {
        active: AtomicUsize,
        peak: Arc<AtomicUsize>,
    }

    impl PasswordHasherService for TracksConcurrency {
        fn hash_password(&self, _: &str) -> Result<String, Error> {
            let active = self.active.fetch_add(1, Ordering::SeqCst) + 1;
            self.peak.fetch_max(active, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(20));
            self.active.fetch_sub(1, Ordering::SeqCst);
            Ok(String::new())
        }

        fn validate_password(&self, _: &str, _: &str) -> Result<bool, Error> {
            Ok(false)
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn concurrent_hashing_is_bounded_even_when_callers_give_up() {
        let limit = PasswordHasher::max_concurrent_async_operations();
        let peak = Arc::new(AtomicUsize::new(0));
        let hasher = PasswordHasher::new(TracksConcurrency {
            active: AtomicUsize::new(0),
            peak: peak.clone(),
        });

        // Abandoned callers must keep holding their slot until the work ends.
        for _ in 0..limit * 2 {
            let hasher = hasher.clone();
            let abandoned = tokio::spawn(async move { hasher.hash_password_async("secret").await });
            tokio::task::yield_now().await;
            abandoned.abort();
        }

        let attempts = (0..limit * 4).map(|_| {
            let hasher = hasher.clone();
            tokio::spawn(async move { hasher.hash_password_async("secret").await })
        });
        for attempt in attempts.collect::<Vec<_>>() {
            attempt.await.unwrap().unwrap();
        }

        let peak = peak.load(Ordering::SeqCst);
        assert!((1..=limit).contains(&peak), "peak {peak} exceeded {limit}");
    }
}
