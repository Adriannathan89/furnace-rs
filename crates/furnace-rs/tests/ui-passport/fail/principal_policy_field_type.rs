use furnace_rs::common::PassportPrincipal;

#[derive(PassportPrincipal)]
struct Principal {
    #[roles]
    roles: u64,
}

fn main() {}
