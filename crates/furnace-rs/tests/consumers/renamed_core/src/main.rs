//! Verifies attribute expansion through a renamed direct-core dependency.

use runtime::AutoConfigurationStatus;

#[derive(runtime::Configuration)]
struct Settings {
    #[config(default = 3000, validate(positive))]
    port: u16,
}

#[runtime::cauldron(providers = [Repository])]
struct AppCauldron;

#[runtime::storage]
struct Repository;

fn framework_result() -> runtime::Result<()> {
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
    assert_eq!(runtime::Config::empty().parse::<Settings>().unwrap().port, 3000);
    let _ = framework_result;
    let _ = status_name;
}

#[runtime::burner]
struct DeclarativeService;
#[runtime::cauldron(providers=[DeclarativeService], exports=[DeclarativeService])]
struct DeclarativeModule;
use DeclarativeModule as DeclarativeAlias;
#[runtime::cauldron(root, imports=[DeclarativeAlias])]
struct DeclarativeApp;
