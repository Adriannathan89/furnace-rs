//! Parse entity-bound storage declarations without coupling core to SeaORM.
use crate::{
    managed::{self, ManagedKind},
    path::persistence_path,
};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use std::collections::BTreeSet;
use syn::{
    Error, ExprPath, Fields, Ident, ItemStruct, Token, Type, TypePath,
    ext::IdentExt,
    parse::{Parse, ParseStream},
    spanned::Spanned,
};

pub(crate) struct Relation {
    pub(crate) alias: Ident,
    pub(crate) selector: ExprPath,
}
pub(crate) struct StorageArgs {
    pub(crate) entity: TypePath,
    pub(crate) connection: Option<Ident>,
    pub(crate) relations: Vec<Relation>,
}
impl Parse for StorageArgs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let mut entity = None;
        let mut connection = None;
        let mut relations = None;
        while !input.is_empty() {
            let name: Ident = input.parse()?;
            match name.to_string().as_str() {
                "entity" => {
                    if entity.is_some() {
                        return Err(Error::new(name.span(), "duplicate storage entity"));
                    }
                    input.parse::<Token![=]>()?;
                    entity = Some(input.parse::<TypePath>()?);
                }
                "connection" => {
                    if connection.is_some() {
                        return Err(Error::new(name.span(), "duplicate storage connection"));
                    }
                    input.parse::<Token![=]>()?;
                    connection = Some(input.parse::<Ident>()?);
                }
                "relations" => {
                    if relations.is_some() {
                        return Err(Error::new(name.span(), "duplicate storage relations"));
                    }
                    let content;
                    syn::parenthesized!(content in input);
                    let mut entries = Vec::new();
                    let mut names = BTreeSet::new();
                    while !content.is_empty() {
                        let alias: Ident = content.parse()?;
                        if !names.insert(alias.unraw().to_string()) {
                            return Err(Error::new(
                                alias.span(),
                                "duplicate storage relation alias",
                            ));
                        }
                        content.parse::<Token![=]>()?;
                        entries.push(Relation {
                            alias,
                            selector: content.parse()?,
                        });
                        if content.is_empty() {
                            break;
                        }
                        content.parse::<Token![,]>()?;
                    }
                    if entries.is_empty() {
                        return Err(Error::new(
                            name.span(),
                            "storage relations must contain at least one loader",
                        ));
                    }
                    relations = Some(entries);
                }
                _ => {
                    return Err(Error::new(
                        name.span(),
                        "unknown storage option; expected entity, connection, or relations",
                    ));
                }
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(Self {
            entity: entity.ok_or_else(|| {
                input.error("entity-bound storage requires entity = path::Entity")
            })?,
            connection,
            relations: relations.unwrap_or_default(),
        })
    }
}

pub(crate) fn expand(arguments: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    if arguments.is_empty() {
        return managed::expand(ManagedKind::Repository, arguments, item);
    }
    let options: StorageArgs = syn::parse2(arguments)?;
    let declaration: ItemStruct = syn::parse2(item.clone())?;
    let managed = managed::expand(ManagedKind::Repository, TokenStream::new(), item)?;
    let connection = select_connection(&options, &declaration)?;
    let persistence = persistence_path()?;
    if !options.relations.is_empty() {
        return Err(Error::new(
            options.relations[0].selector.span(),
            format!(
                "relation loader `{}` is not yet available",
                options.relations[0].alias
            ),
        ));
    }
    let entity = &options.entity;
    let name = &declaration.ident;
    let accessor = quote_spanned! {connection.span()=>
        impl #name {
            /// Borrows typed entity operations using this storage's declared connection.
            pub fn entity(&self)->#persistence::storage::EntityStore<'_,#entity> {
                let database: &#persistence::storage::__sea_orm::DatabaseConnection = &self.0.#connection;
                #persistence::storage::EntityStore::new(database)
            }
        }
    };
    Ok(quote! { #persistence::__furnace_storage_enabled! { #managed #accessor } })
}

fn select_connection(options: &StorageArgs, declaration: &ItemStruct) -> syn::Result<Ident> {
    let Fields::Named(fields) = &declaration.fields else {
        return Err(Error::new(
            declaration.span(),
            "entity-bound storage requires a named injected database field",
        ));
    };
    if let Some(selected) = &options.connection {
        if fields
            .named
            .iter()
            .any(|field| field.ident.as_ref() == Some(selected))
        {
            return Ok(selected.clone());
        }
        return Err(Error::new(
            selected.span(),
            "storage connection must name an existing injected field",
        ));
    }
    let candidates: Vec<_> = fields
        .named
        .iter()
        .filter(|field| {
            let Type::Path(path) = &field.ty else {
                return false;
            };
            path.qself.is_none()
                && path
                    .path
                    .segments
                    .last()
                    .is_some_and(|segment| segment.ident == "DatabaseConnection")
                && path
                    .path
                    .segments
                    .iter()
                    .all(|segment| matches!(segment.arguments, syn::PathArguments::None))
        })
        .filter_map(|field| field.ident.clone())
        .collect();
    if candidates.len() == 1 {
        Ok(candidates[0].clone())
    } else {
        Err(Error::new(
            declaration.ident.span(),
            "storage requires exactly one DatabaseConnection field; use connection = field_name for aliases or multiple connections",
        ))
    }
}
