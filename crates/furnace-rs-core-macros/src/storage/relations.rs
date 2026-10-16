//! Generate scoped native-loader facades for declared relation aliases.
use super::StorageArgs;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, ItemStruct, Path, ext::IdentExt};

pub(super) fn expand(
    options: &StorageArgs,
    declaration: &ItemStruct,
    connection: &Ident,
    persistence: &Path,
) -> TokenStream {
    let name = &declaration.ident;
    let entity = &options.entity;
    let access = format_ident!("__Furnace{}Entity", name);
    let query = format_ident!("__Furnace{}Query", name);
    let flags: Vec<_> = options
        .relations
        .iter()
        .enumerate()
        .map(|(index, _)| format_ident!("__relation_{index}"))
        .collect();
    let methods = options
        .relations
        .iter()
        .zip(&flags)
        .map(|(relation, flag)| {
            let method = format_ident!("with_{}", relation.alias.unraw());
            let documentation = format!(
                "Selects the declared {} relation for native loading.",
                relation.alias.unraw()
            );
            quote! {
                #[doc=#documentation]
                pub fn #method(mut self)->Self {self.#flag=true;self}
            }
        });
    let configure: Vec<_> = options
        .relations
        .iter()
        .zip(&flags)
        .map(|(relation, flag)| {
            let selector = &relation.selector;
            quote! {if self.#flag {loader=loader.with(#selector);}}
        })
        .collect();
    quote! {
        const _: () = {
            #[doc(hidden)]
            pub struct #access<'db> {
                store:#persistence::storage::EntityStore<'db,#entity>,
            }
            impl<'db> ::core::ops::Deref for #access<'db> {
                type Target=#persistence::storage::EntityStore<'db,#entity>;
                fn deref(&self)->&Self::Target {&self.store}
            }
            impl<'db> #access<'db> {
                /// Builds a query with explicitly selected native relation loaders.
                pub fn query(&self)->#query<'db> {
                    #query {base:self.store.query(),#(#flags:false,)*}
                }
            }
            #[doc(hidden)]
            pub struct #query<'db> {
                base:#persistence::storage::EntityQuery<'db,#entity>,
                #(#flags:bool,)*
            }
            impl<'db> #query<'db> {
                /// Adds a native root predicate combined with AND.
                pub fn filter<C:#persistence::storage::__sea_orm::sea_query::IntoCondition>(mut self,condition:C)->Self {
                    self.base=self.base.filter(condition);self
                }
                /// Appends an ascending root-column ordering criterion.
                pub fn order_by_asc(mut self,column:<#entity as #persistence::storage::__sea_orm::EntityTrait>::Column)->Self {
                    self.base=self.base.order_by_asc(column);self
                }
                /// Appends a descending root-column ordering criterion.
                pub fn order_by_desc(mut self,column:<#entity as #persistence::storage::__sea_orm::EntityTrait>::Column)->Self {
                    self.base=self.base.order_by_desc(column);self
                }
                #(#methods)*
                /// Loads one matching root with the selected relations.
                pub async fn one(self)->#persistence::storage::StorageResult<Option<<#entity as #persistence::storage::__sea_orm::EntityTrait>::ModelEx>> {
                    let (database,statement)=self.base.prepare_one();
                    let mut loader=<#entity>::load();
                    #(#configure)*
                    *#persistence::storage::__sea_orm::QueryFilter::query(&mut loader)=statement;
                    loader.one(database).await.map_err(::core::convert::Into::into)
                }
                /// Loads exactly the requested page of roots and selected relations.
                pub async fn paginate(self,page_number:u64,page_size:u64)->#persistence::storage::StorageResult<#persistence::storage::Page<<#entity as #persistence::storage::__sea_orm::EntityTrait>::ModelEx>> {
                    let prepared=self.base.prepare_page(page_number,page_size)?;
                    let mut loader=<#entity>::load();
                    #(#configure)*
                    *#persistence::storage::__sea_orm::QueryFilter::query(&mut loader)=prepared.statement();
                    let items=loader.all(prepared.database()).await?;
                    Ok(prepared.finish(items))
                }
            }
            impl #name {
                /// Borrows entity operations and the declared native relation query API.
                pub fn entity(&self)->#access<'_> {
                    let database:&#persistence::storage::__sea_orm::DatabaseConnection=&self.0.#connection;
                    #access {store:#persistence::storage::EntityStore::new(database)}
                }
            }
        };
    }
}
