use std::sync::{Arc, Mutex};

use furnace_rs::prelude::*;

use super::model::Account;

/// In-memory accounts, so the example needs no database.
///
/// Clones share one store. Replace this type with a `#[storage]` repository to
/// persist accounts; the service does not change.
#[derive(Clone, Default)]
pub struct AccountRepository {
    accounts: Arc<Mutex<Vec<Account>>>,
}

// A plain provider implements `Injector` itself. `#[burner]` and `#[storage]`
// generate this implementation for types with `#[inject]` fields.
impl Injector for AccountRepository {
    type Dependencies = ();

    async fn inject((): Self::Dependencies) -> furnace_rs::core::Result<Self> {
        Ok(Self::default())
    }
}

impl AccountRepository {
    /// Stores a new account, or returns `None` when the username is taken.
    ///
    /// The check and the insert happen under one lock, so two concurrent
    /// registrations cannot both claim a username.
    pub fn insert(&self, username: &str, password_hash: String) -> Option<Account> {
        let mut accounts = self.accounts.lock().expect("account store poisoned");

        if accounts.iter().any(|account| account.username == username) {
            return None;
        }

        let account = Account {
            id: accounts.len() as u64 + 1,
            username: username.to_owned(),
            password_hash,
        };
        accounts.push(account.clone());

        Some(account)
    }

    pub fn find_by_username(&self, username: &str) -> Option<Account> {
        self.find(|account| account.username == username)
    }

    pub fn find_by_id(&self, id: u64) -> Option<Account> {
        self.find(|account| account.id == id)
    }

    fn find(&self, matches: impl Fn(&Account) -> bool) -> Option<Account> {
        let accounts = self.accounts.lock().expect("account store poisoned");

        accounts.iter().find(|account| matches(account)).cloned()
    }
}
