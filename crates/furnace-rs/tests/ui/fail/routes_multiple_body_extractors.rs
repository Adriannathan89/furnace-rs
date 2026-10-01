//! Rejects multiple known FURNACE/Axum body consumers in one route.

use serde::Deserialize;

#[derive(Deserialize, furnace_rs::Input)]
struct CreateUser {
    email: String,
}

#[furnace_rs::routes]
trait Routes {
    #[furnace_rs::post("/")]
    async fn create(
        &self,
        payload: furnace_rs_common::ValidatedJson<CreateUser>,
        request: furnace_rs_common::Request,
    );
}

fn main() {}
