//! Confirms supported route traits and managed controllers compile.

#![deny(missing_docs)]

use furnace_rs::common::{Json, Path};

/// Query application service.
#[furnace_rs::burner]
pub struct QueryUsecase;

/// Command application service.
#[furnace_rs::burner]
pub struct CommandUsecase;

/// Controller with multiple managed dependencies and route contracts.
#[allow(dead_code)]
#[furnace_rs::controller]
pub struct UserController {
    /// Query behavior.
    query: QueryUsecase,
    /// Command behavior.
    command: CommandUsecase,
}

impl ::furnace_rs::Sealable for UserController {
    fn seals() -> ::furnace_rs::SealRegistration<Self> {
        ::furnace_rs::SealRegistration::new()
    }
}

#[furnace_rs::controller]
impl UserController {
    #[furnace_rs::get("/users/:id")]
    async fn get_user(&self, Path(id): Path<i64>) -> String {
        let _query = &self.query;
        id.to_string()
    }
    #[furnace_rs::post("/users")]
    async fn create_user(&self, Json(id): Json<i64>) -> String {
        let _command = &self.command;
        id.to_string()
    }
    #[furnace_rs::put("/users/:id")]
    async fn update_user(&self, Path(id): Path<i64>) -> String {
        id.to_string()
    }
    #[furnace_rs::patch("/users/:id")]
    async fn patch_user(&self, Path(id): Path<i64>) -> String {
        id.to_string()
    }
    #[furnace_rs::delete("/users/:id")]
    async fn delete_user(&self, Path(id): Path<i64>) -> String {
        id.to_string()
    }
}

fn main() {}
