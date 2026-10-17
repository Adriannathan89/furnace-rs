#[furnace_rs::controller]
struct Conditional {
    #[cfg(any())]
    #[inject]
    disabled: MissingDependency,
}

#[furnace_rs::controller]
struct ConditionalAttribute {
    #[cfg_attr(all(), cfg(any()))]
    #[inject]
    disabled: MissingDependency,
}

fn main() {}
