#[mads::module]
struct LegacyModule;
#[mads::service]
struct LegacyService;
#[mads::repository]
struct LegacyRepository;
#[mads::provider]
fn legacy_provider() -> u32 { 1 }
fn main() {
    let _ = mads::core::Mads::run::<LegacyModule>();
}
