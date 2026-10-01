use mads::furnace;

struct NotAModule;

#[furnace]
struct AppModule;

impl mads::core::Furnace for AppModule {
    fn register(self) -> mads::core::FurnaceRegistration<Self> {
        self.import(NotAModule)
    }
}


fn main() {}
