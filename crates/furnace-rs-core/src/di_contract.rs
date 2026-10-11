//! Hidden typed contracts used by declarative cauldron expansions.
#![allow(missing_docs)]
use std::marker::PhantomData;

pub trait ModuleContract: crate::Cauldron {
    type Metadata: ModuleMetadata;
}
pub trait ModuleMetadata {
    const KEY: &'static str;
}
pub trait SlotKey {
    const KEY: &'static str;
}
pub trait SlotAt<const INDEX: usize> {
    type Slot: SlotKey;
}
pub trait ImportAt<const INDEX: usize> {
    type Metadata: ModuleMetadata;
}
pub trait Represents<T> {}
pub trait Lookup<T, Proof> {}
pub trait CheckModule<Scope, Proof> {}
pub trait CheckDependencies<Scope, Proof> {}

pub struct Nil;
pub struct Enabled<const ACTIVE: bool, Slot>(PhantomData<Slot>);
pub struct Leaf<Slot>(PhantomData<Slot>);
pub struct Branch<Left, Right>(PhantomData<(Left, Right)>);
pub struct Here;
pub struct Left<Proof>(PhantomData<Proof>);
pub struct Right<Proof>(PhantomData<Proof>);

pub fn leaf<S>() -> Leaf<S> {
    Leaf(PhantomData)
}
pub fn branch<L, R>(_: L, _: R) -> Branch<L, R> {
    Branch(PhantomData)
}

// At a root these markers are concrete. Autoref dispatch removes explicit
// Config outputs (including aliases) from membership: the intrinsic satisfies
// those inputs. CheckModule still verifies the declared factories' dependencies.
pub struct ConfigFilter<const ACTIVE: bool, Slot>(PhantomData<Slot>);
pub fn config_filter<const ACTIVE: bool, Slot>() -> ConfigFilter<ACTIVE, Slot> {
    ConfigFilter(PhantomData)
}
pub trait FilterConfig {
    type Scope;
    fn filter_config(self) -> Self::Scope;
}
impl<const ACTIVE: bool, Slot> FilterConfig for &ConfigFilter<ACTIVE, Slot> {
    type Scope = Leaf<Enabled<ACTIVE, Slot>>;
    fn filter_config(self) -> Self::Scope {
        leaf()
    }
}
impl<const ACTIVE: bool, Slot: Represents<crate::Config>> FilterConfig
    for &&ConfigFilter<ACTIVE, Slot>
{
    type Scope = Nil;
    fn filter_config(self) -> Self::Scope {
        Nil
    }
}

impl<T, Slot> Lookup<T, Here> for Leaf<Enabled<true, Slot>> where Slot: Represents<T> {}
impl<T, L, R, P> Lookup<T, Left<P>> for Branch<L, R> where L: Lookup<T, P> {}
impl<T, L, R, P> Lookup<T, Right<P>> for Branch<L, R> where R: Lookup<T, P> {}

impl<S> CheckDependencies<S, ()> for () {}
macro_rules! check_tuple {
    ($($ty:ident:$proof:ident),+) => {
        impl<S, $($ty, $proof),+> CheckDependencies<S, ($($proof,)+)> for ($($ty,)+)
        where $(S: Lookup<$ty, $proof>),+ {}
    };
}
check_tuple!(A:PA);
check_tuple!(A:PA,B:PB);
check_tuple!(A:PA,B:PB,C:PC);
check_tuple!(A:PA,B:PB,C:PC,D:PD);
check_tuple!(A:PA,B:PB,C:PC,D:PD,E:PE);
check_tuple!(A:PA,B:PB,C:PC,D:PD,E:PE,F:PF);
check_tuple!(A:PA,B:PB,C:PC,D:PD,E:PE,F:PF,G:PG);
check_tuple!(A:PA,B:PB,C:PC,D:PD,E:PE,F:PF,G:PG,H:PH);
check_tuple!(A:PA,B:PB,C:PC,D:PD,E:PE,F:PF,G:PG,H:PH,I:PI);
check_tuple!(A:PA,B:PB,C:PC,D:PD,E:PE,F:PF,G:PG,H:PH,I:PI,J:PJ);
check_tuple!(A:PA,B:PB,C:PC,D:PD,E:PE,F:PF,G:PG,H:PH,I:PI,J:PJ,K:PK);
check_tuple!(A:PA,B:PB,C:PC,D:PD,E:PE,F:PF,G:PG,H:PH,I:PI,J:PJ,K:PK,L:PL);
check_tuple!(A:PA,B:PB,C:PC,D:PD,E:PE,F:PF,G:PG,H:PH,I:PI,J:PJ,K:PK,L:PL,M:PM);
check_tuple!(A:PA,B:PB,C:PC,D:PD,E:PE,F:PF,G:PG,H:PH,I:PI,J:PJ,K:PK,L:PL,M:PM,N:PN);
check_tuple!(A:PA,B:PB,C:PC,D:PD,E:PE,F:PF,G:PG,H:PH,I:PI,J:PJ,K:PK,L:PL,M:PM,N:PN,O:PO);
check_tuple!(A:PA,B:PB,C:PC,D:PD,E:PE,F:PF,G:PG,H:PH,I:PI,J:PJ,K:PK,L:PL,M:PM,N:PN,O:PO,P:PP);

pub struct ConfigSlot;
impl Represents<crate::Config> for ConfigSlot {}
impl SlotKey for ConfigSlot {
    const KEY: &'static str = "furnace:intrinsic:Config";
}

// Keys only choose candidate marker positions. Identity assertions below must
// accompany any equality/deduplication; keys never stand in for Rust types.
pub const fn same(a: &str, b: &str) -> bool {
    let a = a.as_bytes();
    let b = b.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}
pub const fn first_visible(keys: &[&str], visible: &[bool], index: usize) -> bool {
    if !visible[index] {
        return false;
    }
    let mut i = 0;
    while i < index {
        if visible[i] && same(keys[i], keys[index]) {
            return false;
        }
        i += 1;
    }
    true
}
pub const fn direct_import(provider: &str, imports: &[&str]) -> bool {
    let mut i = 0;
    while i < imports.len() {
        if same(imports[i], provider) {
            return true;
        }
        i += 1;
    }
    false
}
pub trait SameWhen<const MATCH: bool, A, B> {}
impl<A, B> SameWhen<false, A, B> for () {}
impl<A> SameWhen<true, A, A> for () {}
pub fn assert_identity<const MATCH: bool, A, B>()
where
    (): SameWhen<MATCH, A, B>,
{
}
pub fn assert_module<M, S, P>()
where
    M: CheckModule<S, P>,
{
}
pub fn assert_has<T, S, P>()
where
    S: Lookup<T, P>,
{
}
pub fn assert_module_scope<M, S, P>(_: S)
where
    M: CheckModule<S, P>,
{
}
