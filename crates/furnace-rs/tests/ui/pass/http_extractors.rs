//! Confirms route handlers accept standard and native Axum extractors.

#![deny(missing_docs)]

use furnace_rs::common::{Header, Json, Path, Query, Request, ValidatedJson, headers};
use furnace_rs::prelude::*;
use serde::{Deserialize, Serialize};

/// A request path and body fixture.
#[derive(Deserialize, Serialize)]
struct User {
    /// Stable user identifier.
    id: u64,
}

/// A query-string fixture.
#[derive(Deserialize)]
struct SearchQuery {
    /// Requested page number.
    page: u64,
}

/// A JSON request body validated before dispatch.
#[derive(Deserialize, furnace_rs::Input)]
struct CreateUser {
    /// User name received in the request body.
    name: String,
}

/// A controller demonstrating extractor forwarding.
#[controller]
struct ExtractorController;

impl ::furnace_rs::Sealable for ExtractorController {
    fn seals() -> ::furnace_rs::SealRegistration<Self> {
        ::furnace_rs::SealRegistration::new()
    }
}

#[furnace_rs::controller]
impl ExtractorController {
    #[get("/users/:id")]
    async fn get_user(
        &self,
        Path(id): Path<u64>,
        Query(query): Query<SearchQuery>,
        Header(agent): Header<headers::UserAgent>,
        furnace_rs::common::axum::extract::Extension(extension): furnace_rs::common::axum::extract::Extension<
            String,
        >,
        request: Request,
    ) -> Json<User> {
        let _ = (query.page, agent, extension, request);
        Json(User { id })
    }
    #[furnace_rs::post("/users/:id")]
    async fn create_user(
        &self,
        Path(id): Path<u64>,
        Query(query): Query<SearchQuery>,
        Header(agent): Header<headers::UserAgent>,
        Json(request): Json<CreateUser>,
    ) -> Json<User> {
        let _ = (query.page, agent, request.name);
        Json(User { id })
    }
    #[furnace_rs::post("/users/:id/validated")]
    async fn create_validated_user(
        &self,
        Path(id): Path<u64>,
        Query(query): Query<SearchQuery>,
        Header(agent): Header<headers::UserAgent>,
        ValidatedJson(request): ValidatedJson<CreateUser>,
    ) -> Json<User> {
        let _ = (query.page, agent, request.name);
        Json(User { id })
    }
}

fn main() {}
