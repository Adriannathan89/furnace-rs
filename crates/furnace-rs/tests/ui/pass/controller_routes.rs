//! Confirms supported route traits and managed controllers compile.

#![deny(missing_docs)]

use furnace_rs::common::{Json, Path};

/// Query application service.
#[furnace_rs::burner]
pub struct QueryUsecase;

/// Command application service.
#[furnace_rs::burner]
pub struct CommandUsecase;

/// Query route contract.
#[furnace_rs::routes(prefix = "/users")]
pub trait QueryRoutes {
    /// Gets one user.
    #[furnace_rs::get("/:id")]
    async fn get_user(&self, id: Path<i64>) -> String;

    /// Compile-time-disabled route used to verify `cfg` propagation.
    #[cfg(any())]
    #[furnace_rs::get("/cfg-disabled")]
    async fn cfg_disabled(&self);

    /// Compile-time-disabled route used to verify `cfg_attr` propagation.
    #[cfg_attr(all(), cfg(any()))]
    #[furnace_rs::get("/cfg-attr-disabled")]
    async fn cfg_attr_disabled(&self);
}

/// Command route contract.
#[furnace_rs::routes]
pub trait CommandRoutes {
    /// Creates one user.
    #[furnace_rs::post("/users")]
    async fn create_user(&self, id: Json<i64>) -> String;

    /// Updates one user.
    #[furnace_rs::put("/users/:id")]
    async fn update_user(&self, id: Path<i64>) -> String;

    /// Patches one user.
    #[furnace_rs::patch("/users/:id")]
    async fn patch_user(&self, id: Path<i64>) -> String;

    /// Deletes one user.
    #[furnace_rs::delete("/users/:id")]
    async fn delete_user(&self, id: Path<i64>) -> String;
}

/// Controller with multiple managed dependencies and route contracts.
#[allow(dead_code)]
#[furnace_rs::controller(routes = [QueryRoutes, CommandRoutes])]
pub struct UserController {
    /// Query behavior.
    query: QueryUsecase,
    /// Command behavior.
    command: CommandUsecase,
}

impl QueryRoutes for UserController {
    async fn get_user(&self, Path(id): Path<i64>) -> String {
        let _query = &self.query;
        id.to_string()
    }
}

impl CommandRoutes for UserController {
    async fn create_user(&self, Json(id): Json<i64>) -> String {
        let _command = &self.command;
        id.to_string()
    }

    async fn update_user(&self, Path(id): Path<i64>) -> String {
        id.to_string()
    }

    async fn patch_user(&self, Path(id): Path<i64>) -> String {
        id.to_string()
    }

    async fn delete_user(&self, Path(id): Path<i64>) -> String {
        id.to_string()
    }
}

fn main() {}
