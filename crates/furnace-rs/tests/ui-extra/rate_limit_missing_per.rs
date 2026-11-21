use furnace_rs::prelude::*;

#[controller]
struct C;
impl Sealable for C {
    fn seals() -> SealRegistration<Self> {
        SealRegistration::new()
    }
}

#[controller(route = "/x")]
impl C {
    #[get("/a")]
    #[rate_limit(limit = 5)]
    async fn a(&self) -> &'static str {
        "a"
    }
}

fn main() {}
