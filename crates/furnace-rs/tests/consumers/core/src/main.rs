//! Verifies attribute expansion through a direct core dependency.

use furnace_rs_core::AutoConfigurationStatus;

#[furnace_rs_core::cauldron]
struct AppCauldron;

impl furnace_rs_core::Cauldron for AppCauldron {
    fn register(self) -> furnace_rs_core::CauldronRegistration<Self> {
        self.provide::<Repository>()
    }
}


#[furnace_rs_core::storage]
struct Repository;

fn framework_result() -> furnace_rs_core::Result<()> {
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
