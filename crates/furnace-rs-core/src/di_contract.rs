//! Hidden typed contracts used by declarative cauldron expansions.
#![allow(missing_docs)]
use std::marker::PhantomData;

pub trait ModuleContract: Send + Sync + Sized + 'static {
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

/// Creates the leaf marker used to represent one slot in a compile-time scope.
pub fn leaf<S>() -> Leaf<S> {
    Leaf(PhantomData)
}

/// Combines two marker trees into a type-level scope branch.
pub fn branch<L, R>(_: L, _: R) -> Branch<L, R> {
    Branch(PhantomData)
}

// At a root these markers are concrete. Autoref dispatch removes explicit
// Config outputs (including aliases) from membership: the intrinsic satisfies
// those inputs. CheckModule still verifies the declared factories' dependencies.
pub struct ConfigFilter<const ACTIVE: bool, Slot>(PhantomData<Slot>);

/// Creates the autoref-dispatch marker that removes intrinsic Config slots.
pub fn config_filter<const ACTIVE: bool, Slot>() -> ConfigFilter<ACTIVE, Slot> {
    ConfigFilter(PhantomData)
}

pub trait FilterConfig {
    type Scope;

    /// Converts the selected autoref implementation into its filtered scope marker.
    fn filter_config(self) -> Self::Scope;
}

impl<const ACTIVE: bool, Slot> FilterConfig for &ConfigFilter<ACTIVE, Slot> {
    type Scope = Leaf<Enabled<ACTIVE, Slot>>;

    /// Keeps the slot when no intrinsic Config binding matches it.
    fn filter_config(self) -> Self::Scope {
        leaf()
    }
}

impl<const ACTIVE: bool, Slot: Represents<crate::Config>> FilterConfig
    for &&ConfigFilter<ACTIVE, Slot>
{
    type Scope = Nil;

    /// Erases a Config slot because the root supplies Config intrinsically.
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
/// Compares two keys in const contexts without relying on runtime string APIs.
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

/// Selects the earliest visible key with this value for compile-time deduplication.
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

/// Checks whether one module key appears in the direct import list.
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

/// Requires two candidate types to be identical exactly when `MATCH` is true.
pub fn assert_identity<const MATCH: bool, A, B>()
where
    (): SameWhen<MATCH, A, B>,
{
}

/// Produces a trait error when module `M` does not satisfy scope contract `S`.
pub fn assert_module<M, S, P>()
where
    M: CheckModule<S, P>,
{
}

/// Produces a trait error when `T` cannot be looked up in scope `S`.
pub fn assert_has<T, S, P>()
where
    S: Lookup<T, P>,
{
}

/// Checks a module's complete provider/dependency contract against its scope.
pub fn assert_module_scope<M, S, P>(_: S)
where
    M: CheckModule<S, P>,
{
}
