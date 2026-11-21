use std::time::Duration;

use furnace_rs::prelude::*;

use super::{
    model::{Account, AccountClaims, AccountPrincipal},
    repository::AccountRepository,
};

/// Application use cases. The controller only translates HTTP to these calls.
#[burner]
pub struct AccountService {
    #[inject]
    repository: AccountRepository,
    #[inject]
    hasher: PasswordHasher,
    #[inject]
    jwt: JwtService,
}

impl AccountService {
    /// Creates an account, or returns `None` when the username is taken.
    pub async fn register(
        &self,
        username: &str,
        password: &str,
    ) -> PasswordHasherResult<Option<Account>> {
        // The `_async` methods run Argon2 on the blocking pool, so hashing
        // never stalls the runtime threads serving other requests.
        let password_hash = self.hasher.hash_password_async(password).await?;

        Ok(self.repository.insert(username, password_hash))
    }

    /// Returns the account when the username exists and the password matches.
    pub async fn authenticate(
        &self,
        username: &str,
        password: &str,
    ) -> PasswordHasherResult<Option<Account>> {
        let account = self.repository.find_by_username(username);
        let stored_hash = account
            .as_ref()
            .map(|account| account.password_hash.as_str());

        // Pass the lookup result straight through: an unknown username still
        // costs one verification, so response time does not reveal which
        // usernames exist.
        let matches = self
            .hasher
            .validate_optional_password_async(password, stored_hash)
            .await?;

        Ok(account.filter(|_| matches))
    }

    /// Signs a fifteen-minute access token for an authenticated account.
    pub fn issue_token(&self, account: &Account) -> JwtResult<String> {
        self.jwt.sign(
            AccountClaims {
                account_id: account.id,
            },
            JwtSignOptions::access(Duration::from_secs(15 * 60)).subject(account.id.to_string()),
        )
    }

    pub fn find_by_id(&self, id: u64) -> Option<Account> {
        self.repository.find_by_id(id)
    }
}

/// Turns verified token claims into the principal handlers receive.
///
/// The framework checks the signature, expiry, and token kind before
/// `validate` runs. Looking the account up again rejects tokens whose account
/// no longer exists.
#[burner]
pub struct AccountJwtStrategy {
    #[inject]
    accounts: AccountService,
}

#[passport_strategy(name = "jwt")]
impl PassportStrategy for AccountJwtStrategy {
    type Claims = AccountClaims;
    type Principal = AccountPrincipal;
    const TOKEN_KIND: JwtTokenKind = JwtTokenKind::Access;

    async fn validate(
        &self,
        _context: &PassportContext<'_>,
        claims: &JwtClaims<Self::Claims>,
    ) -> PassportResult<Self::Principal> {
        self.accounts
            .find_by_id(claims.custom.account_id)
            .map(|account| AccountPrincipal {
                id: account.id,
                username: account.username,
                roles: vec!["member".to_owned()],
            })
            .ok_or_else(PassportError::reject)
    }
}
