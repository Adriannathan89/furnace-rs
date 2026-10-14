mod controller;
mod model;
mod repository;

use furnace_rs::prelude::*;

#[cauldron(providers = [repository::PostRepository], controllers = [controller::PostController])]
pub struct PostCauldron;
