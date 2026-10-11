//! Root-local metadata collection. State travels through macro tokens rather
//! than process globals or source scanning, including across crate boundaries.
use super::declaration::Declaration;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, quote_spanned};
use syn::{
    Ident, Index, ItemStruct, LitBool, LitStr, Path, Token, Type, bracketed,
    ext::IdentExt,
    parenthesized,
    parse::{Parse, ParseStream},
};

pub(super) fn expand(d: &Declaration, item: &ItemStruct, core: &Path) -> syn::Result<TokenStream> {
    let ident = &item.ident;
    let span = ident.span().unwrap();
    let uid = format!(
        "{}:{}:{}:{}:{}",
        std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default(),
        span.file(),
        span.line(),
        span.column(),
        ident
    );
    let uid = LitStr::new(&uid, ident.span());
    let meta = format_ident!("__Furnace{}Metadata", ident.unraw());
    let private: Path = syn::parse_quote!(#core::__private);
    let outputs = d
        .providers
        .iter()
        .map(|p| &p.output)
        .chain(d.controllers.iter())
        .chain(d.exports.iter())
        .collect::<Vec<_>>();
    let slots = outputs.iter().enumerate().map(|(i, output)| {
        let slot = format_ident!("__Furnace{}Slot{}", ident.unraw(), i);
        let index = Index::from(i);
        let key = LitStr::new(&format!("{}:slot:{i}", uid.value()), ident.span());
        quote! {
            #[doc(hidden)] pub struct #slot;
            impl #private::SlotKey for #slot { const KEY: &'static str = #key; }
            impl #private::Represents<#output> for #slot {}
            impl #private::SlotAt<#index> for #meta { type Slot = #slot; }
        }
    });
    let imports = d.imports.iter().enumerate().map(|(i, import)| {
        let index = Index::from(i);
        quote! {
            impl<M: #private::ModuleMetadata> #private::ImportAt<#index> for #meta
            where #import: #private::ModuleContract<Metadata = M> {
                type Metadata = M;
            }
        }
    });
    let constructors = d
        .providers
        .iter()
        .map(|p| {
            let output = &p.output;
            let injector = p.injector.as_ref().unwrap_or(output);
            quote!(<#injector as #core::Injector<#output>>::Dependencies)
        })
        .chain(
            d.controllers
                .iter()
                .map(|t| quote!(<#t as #core::Injector>::Dependencies)),
        )
        .collect::<Vec<_>>();
    let proofs = (0..constructors.len())
        .map(|i| format_ident!("P{i}"))
        .collect::<Vec<_>>();
    let module_check = if proofs.is_empty() {
        quote!(impl<S> #private::CheckModule<S, ()> for #meta {})
    } else {
        quote! {
            impl<S, #(#proofs),*> #private::CheckModule<S, (#(#proofs,)*)> for #meta
            where #(#constructors: #private::CheckDependencies<S, #proofs>),* {}
        }
    };
    let provider_leaves = (0..d.providers.len()).map(|i| {
        let index = Index::from(i);
        quote!(#private::Leaf<#private::Enabled<true, <#meta as #private::SlotAt<#index>>::Slot>>)
    }).collect::<Vec<_>>();
    let provider_scope = balanced(&provider_leaves, &private);
    let export_checks = d
        .exports
        .iter()
        .map(|t| quote!(#private::assert_has::<#t, #provider_scope, _>();));
    let bootstrap = Bootstrap {
        core: private.clone(),
        ident: ident.clone(),
        uid: uid.clone(),
        members: (0..d.providers.len() + d.controllers.len()).collect(),
        exports: (d.providers.len() + d.controllers.len()..outputs.len()).collect(),
        global: d.global,
        root: d.root,
        pending: d.imports.clone(),
        records: vec![],
    }
    .advance();
    Ok(quote! {
        #[doc(hidden)] pub struct #meta;
        impl #private::ModuleMetadata for #meta { const KEY: &'static str = #uid; }
        impl #private::ModuleContract for #ident { type Metadata = #meta; }
        #(#slots)* #(#imports)* #module_check
        const _: fn() = || { #(#export_checks)* };
        #bootstrap
    })
}

// Child descriptions are captured at the parent's definition scope. They
// contain only opaque metadata projections, so later consumers never resolve
// private source paths or invoke macro-export helpers through crate-root paths.
struct Bootstrap {
    core: Path,
    ident: Ident,
    uid: LitStr,
    members: Vec<usize>,
    exports: Vec<usize>,
    global: bool,
    root: bool,
    pending: Vec<Path>,
    records: Vec<Record>,
}
fn indices(input: ParseStream) -> syn::Result<Vec<usize>> {
    let list;
    parenthesized!(list in input);
    Ok(list
        .parse_terminated(Index::parse, Token![,])?
        .iter()
        .map(|i| i.index as usize)
        .collect())
}
impl Parse for Bootstrap {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let core = input.parse()?;
        input.parse::<Token![;]>()?;
        let ident = input.parse()?;
        input.parse::<Token![;]>()?;
        let uid = input.parse()?;
        let members = indices(input)?;
        let exports = indices(input)?;
        let global = input.parse::<LitBool>()?.value;
        let root = input.parse::<LitBool>()?.value;
        let pending;
        bracketed!(pending in input);
        let pending = pending
            .parse_terminated(Path::parse, Token![,])?
            .into_iter()
            .collect();
        let records = records(input)?;
        Ok(Self {
            core,
            ident,
            uid,
            members,
            exports,
            global,
            root,
            pending,
            records,
        })
    }
}
impl Bootstrap {
    fn tokens(&self) -> TokenStream {
        let Self {
            core,
            ident,
            uid,
            global,
            root,
            pending,
            records,
            ..
        } = self;
        let members = self.members.iter().map(|i| Index::from(*i));
        let exports = self.exports.iter().map(|i| Index::from(*i));
        let records = records.iter().map(Record::tokens);
        quote!(#core; #ident; #uid (#(#members),*) (#(#exports),*) #global #root [#(#pending),*] [#(#records)*])
    }
    fn advance(mut self) -> TokenStream {
        if self.pending.is_empty() {
            return self.finish();
        }
        // The original import position is retained even when a child's graph
        // contains several records. Each child reply advances exactly one import.
        let index = self.records.iter().filter(|r| r.direct).count();
        let index = Index::from(index);
        let next = self.pending.remove(0);
        let core = &self.core;
        let state = self.tokens();
        quote!(#next!(@describe #core::__di_bootstrap, [__FurnaceContract], <__FurnaceModule as __FurnaceContract::ImportAt<#index>>::Metadata, [#state]);)
    }
    fn finish(self) -> TokenStream {
        let Self {
            core,
            ident,
            uid,
            global,
            root,
            ..
        } = &self;
        let hash = uid.value().bytes().fold(0xcbf29ce484222325u64, |h, b| {
            (h ^ u64::from(b)).wrapping_mul(0x100000001b3)
        });
        let callback = format_ident!("__furnace_di_{hash:016x}", span = ident.span());
        let meta = format_ident!("__Furnace{}Metadata", ident.unraw());
        let members = self.members.iter().map(|i| Index::from(*i));
        let exports = self.exports.iter().map(|i| Index::from(*i));
        let imports = self
            .records
            .iter()
            .filter(|r| r.direct)
            .map(|r| &r.module)
            .collect::<Vec<_>>();
        let imports = substitute(quote!(#((#imports))*), ident);
        let mut dedup_guards = TokenStream::new();
        let mut unique: Vec<&Record> = vec![];
        for record in &self.records {
            if let Some(previous) = unique.iter().find(|p| p.uid.value() == record.uid.value()) {
                let a = &previous.module;
                let b = &record.module;
                let assertion = quote!(#core::assert_identity::<true, #a, #b>(););
                dedup_guards.extend(replace_placeholders(
                    assertion,
                    &quote!(#meta),
                    &quote!(#core),
                ));
            } else {
                unique.push(record);
            }
        }
        let children = unique
            .iter()
            .map(|r| {
                let mut t = r.tokens();
                t = substitute(t, ident);
                t
            })
            .collect::<Vec<_>>();
        let checks = if *root {
            quote!(#callback!(@describe #core::__di_collect, [#core], #meta, [#core]);)
        } else {
            quote!()
        };
        quote_spanned! {ident.span()=>
            #[doc(hidden)]
            #[macro_export]
            macro_rules! #callback {
                (@describe $cb:path, [$($contract:tt)*], $module:ty, [$($state:tt)*]) => {
                    $cb! { [$($state)*] [
                        uid(#uid) module($module) members(#(#members),*) exports(#(#exports),*) global(#global) root(#root) direct(false) imports(#imports)
                        #(#children)*
                    ] }
                };
            }
            #[doc(hidden)]
            #[allow(unused_imports)]
            pub use #callback as #ident;
            const _: fn() = || { #dedup_guards };
            #checks
        }
    }
}
// Replace only reserved protocol placeholders in macro data. This is token
// substitution, not type matching: all membership remains Rust trait checking.
fn substitute(tokens: TokenStream, ident: &Ident) -> TokenStream {
    replace_placeholders(
        tokens,
        &quote_spanned!(ident.span()=> $module),
        &quote_spanned!(ident.span()=> $($contract)*),
    )
}
fn replace_placeholders(
    tokens: TokenStream,
    module: &TokenStream,
    contract: &TokenStream,
) -> TokenStream {
    use proc_macro2::{Group, TokenTree};
    tokens
        .into_iter()
        .flat_map(|token| match token {
            TokenTree::Ident(ref i) if i == "__FurnaceModule" => {
                module.clone().into_iter().collect::<Vec<_>>()
            }
            TokenTree::Ident(ref i) if i == "__FurnaceContract" => {
                contract.clone().into_iter().collect()
            }
            TokenTree::Group(g) => {
                let mut group = Group::new(
                    g.delimiter(),
                    replace_placeholders(g.stream(), module, contract),
                );
                group.set_span(g.span());
                vec![TokenTree::Group(group)]
            }
            other => vec![other],
        })
        .collect()
}
struct BootstrapReply {
    state: Bootstrap,
    records: Vec<Record>,
}
impl Parse for BootstrapReply {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let state;
        bracketed!(state in input);
        Ok(Self {
            state: state.parse()?,
            records: records(input)?,
        })
    }
}
pub(crate) fn bootstrap(input: TokenStream) -> syn::Result<TokenStream> {
    let mut reply: BootstrapReply = syn::parse2(input)?;
    for (i, mut record) in reply.records.into_iter().enumerate() {
        record.direct = i == 0;
        reply.state.records.push(record);
    }
    Ok(reply.state.advance())
}

struct Record {
    uid: LitStr,
    module: Type,
    members: Vec<usize>,
    exports: Vec<usize>,
    global: bool,
    root: bool,
    direct: bool,
    imports: Vec<Type>,
}
impl Parse for Record {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let _: Ident = input.parse()?;
        let u;
        parenthesized!(u in input);
        let uid = u.parse()?;
        let _: Ident = input.parse()?;
        let m;
        parenthesized!(m in input);
        let module = m.parse()?;
        let _: Ident = input.parse()?;
        let members = indices(input)?;
        let _: Ident = input.parse()?;
        let exports = indices(input)?;
        let _: Ident = input.parse()?;
        let g;
        parenthesized!(g in input);
        let global = g.parse::<LitBool>()?.value;
        let _: Ident = input.parse()?;
        let r;
        parenthesized!(r in input);
        let root = r.parse::<LitBool>()?.value;
        let _: Ident = input.parse()?;
        let d;
        parenthesized!(d in input);
        let direct = d.parse::<LitBool>()?.value;
        let _: Ident = input.parse()?;
        let list;
        parenthesized!(list in input);
        let mut imports = vec![];
        while !list.is_empty() {
            let t;
            parenthesized!(t in list);
            imports.push(t.parse()?);
        }
        Ok(Self {
            uid,
            module,
            members,
            exports,
            global,
            root,
            direct,
            imports,
        })
    }
}
impl Record {
    fn tokens(&self) -> TokenStream {
        let Self {
            uid,
            module,
            global,
            root,
            direct,
            ..
        } = self;
        let members = self.members.iter().map(|i| Index::from(*i));
        let exports = self.exports.iter().map(|i| Index::from(*i));
        let imports = &self.imports;
        quote!(uid(#uid) module(#module) members(#(#members),*) exports(#(#exports),*) global(#global) root(#root) direct(#direct) imports(#((#imports))*))
    }
}
fn records(input: ParseStream) -> syn::Result<Vec<Record>> {
    let list;
    bracketed!(list in input);
    let mut records = vec![];
    while !list.is_empty() {
        records.push(list.parse()?);
    }
    Ok(records)
}
struct Collection {
    core: Path,
    records: Vec<Record>,
}
impl Parse for Collection {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let core;
        bracketed!(core in input);
        Ok(Self {
            core: core.parse()?,
            records: records(input)?,
        })
    }
}
pub(crate) fn collect(input: TokenStream) -> syn::Result<TokenStream> {
    let state: Collection = syn::parse2(input)?;
    for record in state.records.iter().skip(1) {
        if record.root {
            return Err(syn::Error::new_spanned(
                &record.module,
                "a marked root cauldron cannot be imported into another marked root",
            ));
        }
    }
    Ok(emit_checks(state))
}

fn emit_checks(state: Collection) -> TokenStream {
    let core = &state.core;
    let mut checks = TokenStream::new();
    for (i, a) in state.records.iter().enumerate() {
        let am = &a.module;
        for b in state.records.iter().take(i) {
            let bm = &b.module;
            if a.uid.value() == b.uid.value() {
                checks.extend(quote!(#core::assert_identity::<true, #am, #bm>();));
            }
            checks.extend(quote!(#core::assert_identity::<{#core::same(<#am as #core::ModuleMetadata>::KEY, <#bm as #core::ModuleMetadata>::KEY)}, #am, #bm>();));
        }
    }
    for (scope_index, owner) in state.records.iter().enumerate() {
        let owner_type = &owner.module;
        let imports = owner
            .imports
            .iter()
            .map(|r| {
                let m = r;
                quote!(<#m as #core::ModuleMetadata>::KEY)
            })
            .collect::<Vec<_>>();
        let mut slots = vec![quote!(#core::ConfigSlot)];
        let mut visible = vec![quote!(true)];
        for record in &state.records {
            let m = &record.module;
            for index in &record.members {
                let i = Index::from(*index);
                slots.push(quote!(<#m as #core::SlotAt<#i>>::Slot));
                visible.push(quote!(#core::same(<#owner_type as #core::ModuleMetadata>::KEY, <#m as #core::ModuleMetadata>::KEY)));
            }
            for index in &record.exports {
                let i = Index::from(*index);
                let global = record.global;
                slots.push(quote!(<#m as #core::SlotAt<#i>>::Slot));
                visible.push(quote!(!#core::same(<#owner_type as #core::ModuleMetadata>::KEY, <#m as #core::ModuleMetadata>::KEY) && (#global || #core::direct_import(<#m as #core::ModuleMetadata>::KEY, &[#(#imports),*]))));
            }
        }
        let k = format_ident!("__FURNACE_KEYS_{scope_index}");
        let v = format_ident!("__FURNACE_VISIBLE_{scope_index}");
        let s = format_ident!("__FurnaceScope{scope_index}");
        let keys = slots.iter().map(|t| quote!(<#t as #core::SlotKey>::KEY));
        let leaves = slots.iter().enumerate().map(|(i, slot)| quote!(#core::Leaf<#core::Enabled<{#core::first_visible(#k, #v, #i)}, #slot>>)).collect::<Vec<_>>();
        let tree = balanced(&leaves, core);
        for (i, a) in slots.iter().enumerate() {
            for b in slots.iter().take(i) {
                checks.extend(quote!(#core::assert_identity::<{#core::same(<#a as #core::SlotKey>::KEY, <#b as #core::SlotKey>::KEY)}, #a, #b>();));
            }
        }
        checks.extend(quote! {
            const #k: &[&str] = &[#(#keys),*];
            const #v: &[bool] = &[#(#visible),*];
            type #s = #tree;
            #core::assert_module::<#owner_type, #s, _>();
        });
    }
    quote!(const _: fn() = || { #checks };)
}
fn balanced(items: &[TokenStream], core: &Path) -> TokenStream {
    match items.len() {
        0 => quote!(#core::Nil),
        1 => items[0].clone(),
        _ => {
            let middle = items.len() / 2;
            let l = balanced(&items[..middle], core);
            let r = balanced(&items[middle..], core);
            quote!(#core::Branch<#l, #r>)
        }
    }
}
