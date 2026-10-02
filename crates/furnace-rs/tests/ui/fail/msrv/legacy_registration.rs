#[furnace_rs::module]
struct LegacyCauldron;
#[furnace_rs::service]
struct LegacyService;
#[furnace_rs::repository]
struct LegacyRepository;
#[furnace_rs::provider]
fn legacy_provider() -> u32 { 1 }
fn main() {
    let _ = furnace_rs::core::Furnace::run::<LegacyCauldron>();
}
