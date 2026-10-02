use furnace_rs::Injector;
struct Service;
impl Injector for Service {
    type Dependencies = (
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
        u8,
    );
    async fn inject(_: Self::Dependencies) -> furnace_rs::core::Result<Self> {
        Ok(Self)
    }
}
fn main() {}
