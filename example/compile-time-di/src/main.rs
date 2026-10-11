use furnace_rs::{Furnace, Logger, LoggerCauldron, burner, cauldron};

#[burner]
struct Greeting {
    #[inject]
    logger: Logger,
}
impl Greeting {
    fn greet(&self) {
        self.logger.info("compile-time DI example is running");
    }
}
#[cauldron(providers=[Greeting], exports=[Greeting])]
struct GreetingCauldron;
#[cauldron(root, imports=[GreetingCauldron, LoggerCauldron])]
struct AppCauldron;

#[tokio::main]
async fn main() -> furnace_rs::core::Result<()> {
    let mut builder = Furnace::builder();
    builder.root::<AppCauldron>()?;
    let mut application = builder.build().await?;
    application.start().await?;
    application.context().resolve::<Greeting>()?.greet();
    application.shutdown().await?;
    Ok(())
}
