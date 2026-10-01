//! Verifies attribute expansion through a direct core dependency.

use mads_core::AutoConfigurationStatus;

#[mads_core::furnace]
struct AppModule;

impl mads_core::Furnace for AppModule {
    fn register(self) -> mads_core::FurnaceRegistration<Self> {
        self.provide::<Repository>()
    }
}


#[mads_core::storage]
struct Repository;

fn framework_result() -> mads_core::Result<()> {
    Ok(())
}

fn status_name(status: AutoConfigurationStatus) -> &'static str {
    match status {
        AutoConfigurationStatus::Active => "active",
        AutoConfigurationStatus::Skipped => "skipped",
        AutoConfigurationStatus::Overridden => "overridden",
        AutoConfigurationStatus::Failed => "failed",
    }
}

fn main() {
    let _ = framework_result;
    let _ = status_name;
}
