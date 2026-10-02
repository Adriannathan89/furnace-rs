use furnace_rs::Configuration;

struct NotConfiguration;

#[derive(Configuration)]
struct Malformed {
    #[config(rename)]
    name: String,
    child: NotConfiguration,
}

fn main() {}
