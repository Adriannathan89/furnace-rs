use furnace::prelude::*;
use furnace::{GrpcClientError, GrpcClientResult};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct DemoClaims {
    pub allowed: bool,
}
impl DemoClaims {
    pub fn reader() -> Self {
        Self { allowed: true }
    }
}
impl PassportPrincipal for DemoClaims {
    fn has_role(&self, role: &str) -> bool {
        self.allowed && role == "reader"
    }
    fn has_permission(&self, permission: &str) -> bool {
        self.allowed && permission == "demo:use"
    }
}
fn allowed(principal: &ClaimsPrincipal<DemoClaims>) -> bool {
    principal.allowed
}
#[guard(strategy = "jwt", principal = ClaimsPrincipal<DemoClaims>, roles(any = ["reader"]), permissions(all = ["demo:use"]), predicate = allowed)]
pub struct Reader;
/// Mints a fresh local demo credential per call; real applications may contact an issuer.
#[derive(Clone)]
pub struct Tokens {
    jwt: JwtService,
    calls: Arc<AtomicUsize>,
}
impl Injector for Tokens {
    type Dependencies = (JwtService,);
    async fn inject((jwt,): Self::Dependencies) -> furnace::core::Result<Self> {
        Ok(Self {
            jwt,
            calls: Arc::default(),
        })
    }
}
impl Tokens {
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}
impl GrpcTokenProvider for Tokens {
    async fn token(&self) -> GrpcClientResult<furnace::core::Secret<String>> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let token = self
            .jwt
            .sign(
                DemoClaims::reader(),
                JwtSignOptions::access(Duration::from_secs(60)),
            )
            .map_err(GrpcClientError::token_provider)?;
        Ok(furnace::core::Secret::new(token))
    }
}
