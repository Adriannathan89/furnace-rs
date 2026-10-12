use furnace_rs_core::__private::*;
struct A;
struct B;
impl ModuleMetadata for A { const KEY: &'static str = "a"; }
impl ModuleMetadata for B { const KEY: &'static str = "b"; }
impl<S> CheckModule<S, ()> for A {}
impl<S> CheckModule<S, ()> for B {}
furnace_rs_core::__private::__di_collect! { [furnace_rs_core::__private] [
uid("a") module(A) members() exports() global(false) root(true) direct(false) imports()
uid("a") module(B) members() exports() global(false) root(false) direct(false) imports()
] }
fn main() {}
