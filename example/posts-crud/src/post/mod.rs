mod controller;
mod model;
mod repository;
mod service;

use mads::prelude::*;

#[furnace]
pub struct PostModule;

impl mads::core::Furnace for PostModule {
    fn register(self) -> mads::core::FurnaceRegistration<Self> {
        self.provide::<service::PostService>()
            .provide::<repository::PostRepository>()
            .controller::<controller::PostController>()
    }
}
