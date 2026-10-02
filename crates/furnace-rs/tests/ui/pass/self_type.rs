//! Confirms nested `Self` field types refer to the generated public handle.

#[furnace_rs::burner]
struct RecursiveService {
    parent: Box<Self>,
}

impl RecursiveService {
    fn parent(&self) -> &Self {
        &self.parent
    }
}

fn main() {
    let _method: fn(&RecursiveService) -> &RecursiveService = RecursiveService::parent;
}
