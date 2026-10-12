//! Type-compatible checks emitted in lexical validator order.

use super::{
    arguments,
    attributes::{Validator, check_number, numeric},
    type_name,
};
use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Error, Generics, Path, Type, parse_quote_spanned, spanned::Spanned};

pub(super) fn expand(
    ty: &Type,
    validators: &[Validator],
    common: &Path,
    generics: &mut Generics,
) -> syn::Result<TokenStream> {
    let ty = super::ungroup(ty);
    let optional = type_name(ty) == "Option";
    let ty = if optional {
        arguments(ty)
            .first()
            .copied()
            .ok_or_else(|| Error::new_spanned(ty, "expected Option<T>"))?
    } else {
        ty
    };
    let ty = super::ungroup(ty);
    let name = type_name(ty);
    let float = matches!(name.as_str(), "f32" | "f64");
    let number = matches!(
        name.as_str(),
        "u8" | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "usize"
            | "i8"
            | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "isize"
            | "f32"
            | "f64"
    );
    let string = name == "String"
        || matches!(ty, Type::Reference(reference) if type_name(&reference.elem) == "str");
    let runtime = quote!(#common::__private::input_validation);
    let mut checks = Vec::new();
    for validator in validators {
        let compatible = match validator.name.as_str() {
            "email" => string,
            "length" | "nonempty" => string || collection(ty)?,
            "range" | "positive" | "multiple_of" => number,
            "negative" => number && !name.starts_with('u'),
            "required" => optional,
            "nested" | "custom" => true,
            _ => false,
        };
        if !compatible {
            return Err(Error::new(
                validator.span,
                "input validator is incompatible with the field type",
            ));
        }
        let mut conditions = Vec::new();
        match validator.name.as_str() {
            "required" => continue,
            "custom" => {
                let callback = validator.callback.as_ref().unwrap();
                checks.push(quote_spanned! {callback.span()=>
                    let __furnace_custom_result: #common::ValidationResult = #callback(__furnace_checked);
                    #runtime::merge(
                        &mut __furnace_errors,
                        __furnace_custom_result
                            .map_err(|errors| errors.__prefix(__furnace_path)),
                    );
                });
            }
            "nested" => {
                // The intrinsic float check already runs once before validators.
                if float {
                    continue;
                }
                let body = nested(ty, quote!(__furnace_checked), common, generics)?;
                checks.push(quote! {
                    #runtime::merge(&mut __furnace_errors, (#body).map_err(|errors| errors.__prefix(__furnace_path)));
                });
            }
            "email" => conditions.push((quote!(!#runtime::email(__furnace_checked)), "Email")),
            "nonempty" => {
                let len = length(ty, &runtime);
                conditions.push((quote!(#len == 0), "TooSmall"));
            }
            "positive" => conditions.push((quote!(*__furnace_checked <= 0 as #ty), "TooSmall")),
            "negative" => conditions.push((quote!(*__furnace_checked >= 0 as #ty), "TooBig")),
            "multiple_of" => {
                let expression = &validator.arguments[0].1;
                check_number(expression, &name)?;
                let value = if float {
                    let number = numeric(expression)?;
                    quote!((#number) as #ty)
                } else {
                    quote!(#expression)
                };
                conditions.push((
                    quote!(!#runtime::Number::multiple_of(*__furnace_checked, #value)),
                    "NotMultipleOf",
                ));
            }
            "length" | "range" => {
                let observed = if validator.name == "length" {
                    length(ty, &runtime)
                } else {
                    quote!(*__furnace_checked)
                };
                for (bound, expression) in &validator.arguments {
                    check_number(
                        expression,
                        if validator.name == "length" {
                            "usize"
                        } else {
                            &name
                        },
                    )?;
                    let value = if float {
                        let number = numeric(expression)?;
                        quote!((#number) as #ty)
                    } else {
                        quote!(#expression)
                    };
                    if bound == "min" || bound == "exact" {
                        conditions.push((quote!(#observed < #value), "TooSmall"));
                    }
                    if bound == "max" || bound == "exact" {
                        conditions.push((quote!(#observed > #value), "TooBig"));
                    }
                }
            }
            _ => unreachable!("parsed validator"),
        }
        for (condition, kind) in conditions {
            let failure = failure(kind, common);
            if float {
                checks.push(quote!(if __furnace_finite && (#condition) { #failure }));
            } else {
                checks.push(quote!(if #condition { #failure }));
            }
        }
    }
    let checks = if float {
        let failure = failure("InvalidType", common);
        // Invalid floats suppress numeric rules, not independent callbacks.
        quote!(let __furnace_finite = #runtime::Number::finite(*__furnace_checked);
            if !__furnace_finite { #failure }
            #(#checks)*)
    } else {
        quote!(#(#checks)*)
    };
    let body = if optional {
        let required = if validators
            .iter()
            .any(|validator| validator.name == "required")
        {
            failure("Required", common)
        } else {
            quote!()
        };
        quote!(if let Some(__furnace_checked) = __furnace_value { #checks } else { #required })
    } else {
        quote!(let __furnace_checked = __furnace_value; #checks)
    };
    Ok(body)
}

fn failure(kind: &str, common: &Path) -> TokenStream {
    let kind = syn::Ident::new(kind, proc_macro2::Span::call_site());
    quote! {
        #common::__private::input_validation::merge(&mut __furnace_errors,
            Err(#common::ValidationErrors::from_issue(#common::__private::input_validation::issue(#common::__private::input_validation::IssueKind::#kind)).__prefix(__furnace_path)));
    }
}

fn length(ty: &Type, runtime: &TokenStream) -> TokenStream {
    if let Type::Tuple(tuple) = ty {
        let len = tuple.elems.len();
        quote!(#len)
    } else {
        quote!(#runtime::Length::length(__furnace_checked))
    }
}

fn collection(ty: &Type) -> syn::Result<bool> {
    let name = type_name(ty);
    if matches!(name.as_str(), "HashMap" | "BTreeMap") {
        if arguments(ty)
            .first()
            .is_none_or(|ty| type_name(ty) != "String")
        {
            return Err(Error::new_spanned(
                ty,
                "automatic input map validation requires String keys",
            ));
        }
        return Ok(true);
    }
    Ok(name == "Vec" || matches!(ty, Type::Array(_) | Type::Tuple(_)))
}

fn nested(
    ty: &Type,
    value: TokenStream,
    common: &Path,
    generics: &mut Generics,
) -> syn::Result<TokenStream> {
    let ty = super::ungroup(ty);
    collection(ty)?;
    let name = type_name(ty);
    let args = arguments(ty);
    if name == "Option" {
        let inner = args
            .first()
            .ok_or_else(|| Error::new_spanned(ty, "expected Option<T>"))?;
        let child = nested(inner, quote!(__furnace_item), common, generics)?;
        return Ok(
            quote!({ match (#value).as_ref() { Some(__furnace_item) => #child, None => Ok(()) } }),
        );
    }
    let sequence = name == "Vec" || matches!(ty, Type::Array(_));
    let map = matches!(name.as_str(), "HashMap" | "BTreeMap");
    if sequence || map {
        let inner = match ty {
            Type::Array(array) => &array.elem,
            _ => args
                .get(usize::from(map))
                .copied()
                .ok_or_else(|| Error::new_spanned(ty, "expected a collection value type"))?,
        };
        let child = nested(inner, quote!(__furnace_item), common, generics)?;
        let (iterator, segment) = if map {
            (
                quote!({ let mut __furnace_entries: Vec<_> = (#value).iter().collect(); __furnace_entries.sort_unstable_by_key(|(key, _)| *key); __furnace_entries }),
                quote!(#common::ValidationPathSegment::Field(__furnace_key.clone())),
            )
        } else {
            (
                quote!((#value).iter().enumerate()),
                quote!(#common::ValidationPathSegment::Index(__furnace_key)),
            )
        };
        return Ok(quote!({
            let mut __furnace_collection_errors = None;
            for (__furnace_key, __furnace_item) in #iterator {
                #common::__private::input_validation::merge(&mut __furnace_collection_errors,
                    (#child).map_err(|errors: #common::ValidationErrors| errors.__prefix(&[#segment])));
            }
            #common::__private::input_validation::finish(__furnace_collection_errors)
        }));
    }
    if let Type::Tuple(tuple) = ty {
        let mut children = Vec::new();
        for (index, ty) in tuple.elems.iter().enumerate() {
            let access = syn::Index::from(index);
            let child = nested(ty, quote!(&__furnace_tuple.#access), common, generics)?;
            children.push(quote! {
                #common::__private::input_validation::merge(&mut __furnace_tuple_errors,
                    (#child).map_err(|errors| errors.__prefix(&[#common::ValidationPathSegment::Index(#index)])));
            });
        }
        return Ok(
            quote!({ let __furnace_tuple = #value; let mut __furnace_tuple_errors = None; #(#children)* #common::__private::input_validation::finish(__furnace_tuple_errors) }),
        );
    }
    if needs_bound(ty, generics) {
        generics
            .make_where_clause()
            .predicates
            .push(parse_quote_spanned!(ty.span()=> #ty: #common::Input));
    }
    Ok(quote_spanned!(ty.span()=> {
        let __furnace_nested: fn(&#ty) -> #common::ValidationResult = <#ty as #common::Input>::validate;
        __furnace_nested(#value)
    }))
}

fn needs_bound(ty: &Type, generics: &Generics) -> bool {
    use syn::visit_mut::VisitMut;
    struct Find<'a> {
        generics: &'a Generics,
        found: bool,
    }
    impl VisitMut for Find<'_> {
        fn visit_type_path_mut(&mut self, path: &mut syn::TypePath) {
            if let Some(first) = path.path.segments.first() {
                self.found |= self
                    .generics
                    .type_params()
                    .any(|parameter| parameter.ident == first.ident);
            }
            syn::visit_mut::visit_type_path_mut(self, path);
        }
    }
    let mut visitor = Find {
        generics,
        found: false,
    };
    visitor.visit_type_mut(&mut ty.clone());
    visitor.found
}
