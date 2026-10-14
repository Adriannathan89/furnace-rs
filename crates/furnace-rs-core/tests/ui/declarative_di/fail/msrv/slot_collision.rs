use furnace_rs_core::__private::*;
struct A;
struct B;
impl ModuleMetadata for A { const KEY: &'static str = "a"; }
impl ModuleMetadata for B { const KEY: &'static str = "b"; }
impl<S> CheckModule<S, ()> for A {}
impl<S> CheckModule<S, ()> for B {}
struct SA;
struct SB;
impl SlotKey for SA { const KEY: &'static str = "collision"; }
impl SlotKey for SB { const KEY: &'static str = "collision"; }
impl SlotAt<0> for A { type Slot = SA; }
impl SlotAt<0> for B { type Slot = SB; }
furnace_rs_core::__private::__di_collect! { [furnace_rs_core::__private] [
uid("a") module(A) members(0) exports() global(false) root(true) direct(false) imports()
uid("b") module(B) members(0) exports() global(false) root(false) direct(false) imports()
] }
fn main() {}
