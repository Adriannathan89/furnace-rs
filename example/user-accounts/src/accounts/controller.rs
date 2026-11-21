use furnace_rs::prelude::*;

use super::{
    model::{AccountPrincipal, AccountResponse, Credentials, TokenResponse},
    service::AccountService,
};

/// Public endpoints: a controller without `impl Sealable` has no guard.
#[controller]
pub struct AccountController {
    #[inject]
    accounts: AccountService,
}

#[controller(route = "/accounts")]
impl AccountController {
    #[post("/register")]
    async fn register(
        &self,
        ValidatedJson(input): ValidatedJson<Credentials>,
    ) -> HttpResult<Json<AccountResponse>> {
        let account = self
            .accounts
            .register(&input.username, &input.password)
            .await
            // Internal errors render a fixed message; the source stays server-side.
            .map_err(InternalError::new)?
            .ok_or_else(|| Conflict::new("username is already taken"))?;

        Ok(Json(AccountResponse {
            id: account.id,
            username: account.username,
        }))
    }

    #[post("/login")]
    async fn login(
        &self,
        ValidatedJson(input): ValidatedJson<Credentials>,
    ) -> HttpResult<Json<TokenResponse>> {
        // One message for both an unknown username and a wrong password.
        let account = self
            .accounts
            .authenticate(&input.username, &input.password)
            .await
            .map_err(InternalError::new)?
            .ok_or_else(|| Unauthorized::new("invalid credentials"))?;
        let access_token = self
            .accounts
            .issue_token(&account)
            .map_err(InternalError::new)?;

        Ok(Json(TokenResponse { access_token }))
    }
}

#[guard(strategy = "jwt", principal = AccountPrincipal, source = bearer, roles(any = ["member"]))]
struct MemberGuard;

/// Protected endpoints: every route requires a valid Bearer token.
#[controller]
pub struct ProfileController {}

impl Sealable for ProfileController {
    fn seals() -> SealRegistration<Self> {
        Self::seal::<MemberGuard>()
    }
}

#[controller(route = "/accounts")]
impl ProfileController {
    #[get("/me")]
    async fn me(
        &self,
        principal: Authenticated<AccountPrincipal>,
    ) -> HttpResult<Json<AccountResponse>> {
        Ok(Json(AccountResponse {
            id: principal.id,
            username: principal.username.clone(),
        }))
    }
}
