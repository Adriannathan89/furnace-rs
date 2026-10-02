use std::sync::Arc;

use furnace_rs::prelude::*;

use super::{
    model::{LoginInput, ProfileResponse, TokenResponse, UserPrincipal},
    traits::AuthService,
};

// Controller: only translates HTTP input/output and invokes the service.
#[controller]
pub struct AuthController {
    service: Arc<dyn AuthService>,
}

#[controller(route = "/auth")]
impl AuthController {
    #[post("/login")]
    async fn login(
        &self,
        ValidatedJson(input): ValidatedJson<LoginInput>,
    ) -> HttpResult<Json<TokenResponse>> {
        let token = self
            .service
            .login(&input.username, &input.password)
            .map_err(InternalError::new)?
            .ok_or_else(|| Unauthorized::new("invalid credentials"))?;
        Ok(Json(TokenResponse {
            access_token: token,
        }))
    }
}

#[guard(strategy = "jwt", principal = UserPrincipal, source = bearer, roles(any = ["reader"]))]
struct ProfileGuard;
#[controller]
pub struct ProfileController {
    logger: Logger,
}
impl Sealable for ProfileController {
    fn seals() -> SealRegistration<Self> {
        Self::seal::<ProfileGuard>()
    }
}
#[controller(route = "/auth")]
impl ProfileController {
    #[get("/me")]
    async fn me(
        &self,
        principal: Authenticated<UserPrincipal>,
    ) -> HttpResult<Json<ProfileResponse>> {
        self.logger.info("protected profile read");
        Ok(Json(ProfileResponse {
            id: principal.id,
            username: principal.username.clone(),
        }))
    }
}
