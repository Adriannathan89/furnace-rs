mod controller;
mod model;
mod repository;
mod service;

use furnace_rs::prelude::*;

#[cauldron]
pub struct PostCauldron;

impl furnace_rs::core::Cauldron for PostCauldron {
    fn register(self) -> furnace_rs::core::CauldronRegistration<Self> {
        self.provide::<service::PostService>()
            .provide::<repository::PostRepository>()
            .controller::<controller::PostController>()
    }
}
