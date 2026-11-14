mod controller;
mod model;
mod repository;
mod service;

#[cfg(test)]
mod tests;

use furnace_rs::prelude::*;

// One feature, one cauldron: everything the accounts feature needs is listed
// here. `PasswordHasherCauldron` supplies the injectable `PasswordHasher`.
#[cauldron(
    imports = [PasswordHasherCauldron],
    providers = [
        repository::AccountRepository,
        JwtService,
        service::AccountService,
        service::AccountJwtStrategy,
    ],
    controllers = [controller::AccountController, controller::ProfileController],
)]
pub struct AccountsCauldron;
