use furnace_rs::prelude::*;
use serde::{Deserialize, Serialize};

/// A stored account. `password_hash` is a PHC string, never the password.
#[derive(Clone)]
pub struct Account {
    pub id: u64,
    pub username: String,
    pub password_hash: String,
}

/// Request body shared by registration and login.
#[derive(Deserialize, Input)]
pub struct Credentials {
    #[validate(length(min = 3, max = 32))]
    pub username: String,
    // An upper bound keeps hashing cost independent of attacker-chosen input.
    #[validate(length(min = 12, max = 128))]
    pub password: String,
}

#[derive(Serialize, Deserialize)]
pub struct AccountClaims {
    pub account_id: u64,
}

#[derive(PassportPrincipal)]
pub struct AccountPrincipal {
    pub id: u64,
    pub username: String,
    #[roles]
    pub roles: Vec<String>,
}

#[derive(Serialize)]
pub struct AccountResponse {
    pub id: u64,
    pub username: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub access_token: String,
}
