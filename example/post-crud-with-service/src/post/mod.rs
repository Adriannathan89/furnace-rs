mod controller;
mod model;
mod repository;
mod service;

use furnace_rs::prelude::*;

#[cauldron(providers = [repository::PostRepository, service::PostService], controllers = [controller::PostController])]
pub struct PostCauldron;
