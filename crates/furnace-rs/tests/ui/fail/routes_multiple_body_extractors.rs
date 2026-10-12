//! Rejects multiple known FURNACE/Axum body consumers in one route.

use serde::Deserialize;

#[derive(Deserialize, furnace_rs::Input)]
struct CreateUser {
    email: String,
}

#[furnace_rs::controller]
struct Routes;
impl furnace_rs::Sealable for Routes {
    fn seals() -> furnace_rs::SealRegistration<Self> { furnace_rs::SealRegistration::new() }
}

#[furnace_rs::controller]
impl Routes {
    #[furnace_rs::post("/")]
    async fn create(
        &self,
        payload: furnace_rs_common::ValidatedJson<CreateUser>,
        request: furnace_rs_common::Request,
    ) {}
}

fn main() {}
