//! Rejects route verbs nested inside conditional attribute expansion.

#[furnace_rs::routes]
trait Routes {
    #[cfg_attr(feature = "conditional-route", furnace_rs::get("/conditional"))]
    async fn conditional(&self) -> &'static str;
}

fn main() {}
