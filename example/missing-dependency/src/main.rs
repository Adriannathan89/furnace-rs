use furnace_rs::{burner, cauldron};

#[burner]
struct OrderRepository;

#[burner]
struct OrderService {
    #[inject]
    repository: OrderRepository,
}

// Intentionally missing OrderRepository. Add it to providers to fix the error.
#[cauldron(root, providers = [OrderService])]
struct AppCauldron;

// The root is checked even without building or starting an application.
fn main() {}
