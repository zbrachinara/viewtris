#![feature(let_else)]

use std::collections::HashMap;

use itertools::{izip, Itertools};
use proc_macro::TokenStream;
use proc_macro2::{Ident, TokenStream as TokenStream2, TokenTree};
use quote::{quote, ToTokens};
use syn::{Data, DeriveInput, Fields, FieldsNamed};
use tap::{Pipe, Tap};
// use venial::{Declaration, StructFields, NamedStructFields};

#[proc_macro_attribute]
pub fn hybrid_tagged(attr: TokenStream, item: TokenStream) -> TokenStream {
    println!("{attr}");
    println!("{item}");

    hybrid_tagged_impl(attr.into(), item.into()).into()
}

fn hybrid_tagged_impl(attr: TokenStream2, item: TokenStream2) -> TokenStream2 {
    let tagged_type: DeriveInput = syn::parse2(item).unwrap();
    let Data::Enum(tagged_enum) = tagged_type.data else {
        panic!("hybrid_tagged is meant to be invoked on an enum")
    };

    let args = attr_args(attr);

    let ret = {
        let common_fields = args
            .get("fields")
            .expect("Argument `fields` was not provided to the proc macro")
            .pipe(|tokens| syn::parse2::<FieldsNamed>(tokens.to_token_stream()).unwrap());
        let common_fields_inner = &common_fields.named;
        let tag = args.get("tag").unwrap().to_token_stream();
        let variants = tagged_enum.variants;
        let name = tagged_type.ident;

        let module_name = Ident::new(
            &format!("{}_data", name.to_string().to_lowercase()),
            name.span(),
        );

        let raw_name_str = format!("Raw{name}");
        let raw_name = Ident::new(&raw_name_str, name.span());

        let data_enum_name = Ident::new(&format!("{name}Data"), name.span());
        let original_attrs = tagged_type.attrs;

        let visibility = tagged_type.vis;

        // takes the variants of the annotated enum and adds the common fields to each one
        let raw_variants = variants.clone().tap_mut(|variants| {
            variants
                .iter_mut()
                .for_each(|variant| match variant.fields {
                    Fields::Named(ref mut f) => common_fields
                        .named
                        .iter()
                        .cloned()
                        .for_each(|field| f.named.push(field)),
                    Fields::Unit => variant.fields = Fields::Named(common_fields.clone()),
                    _ => panic!("Fields of this enum must be named"),
                })
        });

        // raw enum which serde directly translates from json
        let raw_enum = quote!(
            #[serde(tag=#tag)] #(#original_attrs)* enum #raw_name {
                #raw_variants
            }
        );

        // public-facing struct which takes the place of the annotated enum
        let public_struct = quote!(
            #[derive(Serialize, Deserialize)]
            #[serde(from = #raw_name_str, into = #raw_name_str)]
            pub struct #name {
                data: #data_enum_name,
                #common_fields_inner
            }
        );

        let common_fields_names = common_fields_inner
            .iter()
            .cloned()
            .map(|field| field.ident.expect("Fields of this enum must be named"))
            .collect_vec();
        let variant_fields_names = variants
            .iter()
            .map(|variant| {
                variant
                    .fields
                    .iter()
                    .map(|field| field.ident.clone().unwrap())
                    .collect_vec()
            })
            .collect_vec();
        let raw_fields_names = raw_variants
            .iter()
            .map(|variant| {
                variant
                    .fields
                    .iter()
                    .map(|field| field.ident.clone().unwrap())
                    .collect_vec()
            })
            .collect_vec();

        let variant_names = variants.iter().cloned().map(|variant| variant.ident);

        let (convert_from_raw, convert_to_raw): (Vec<_>, Vec<_>) =
            izip!(variant_fields_names, raw_fields_names)
                .zip(variant_names)
                .map(|((variant, raw), branch_name)| {
                    let from_raw = quote!(
                        #raw_name :: #branch_name { #(#raw),* } => Self {
                            data: #data_enum_name :: #branch_name {
                                #(#variant),*
                            }, #(#common_fields_names),*
                        }
                    );

                    let to_raw = quote!(
                        #name :: #branch_name {
                            data: #data_enum_name :: #branch_name { #(#variant),*},
                            #(#common_fields_names),*
                        } => Self :: #branch_name {
                            #(#raw),*
                        }
                    );

                    (from_raw, to_raw)
                })
                .unzip();

        // From impls for converting to and from the public struct and private type
        let convert_impls = quote!(
            impl From<#raw_name> for #name {
                fn from(f: #raw_name) -> Self {
                    match f {
                        #(#convert_from_raw),*
                    }
                }
            }

            impl From<#name> for #raw_name {
                fn from(f: #name) -> Self {
                    match f {
                        #(#convert_to_raw),*
                    }
                }
            }
        );

        // println!("{}", convert_impls);

        // data enum containing the specific data for each variant
        let data_enum = quote!(
            enum #data_enum_name {
                #variants
            }
        );

        // all put together
        quote!(
            #visibility use #module_name::#name;
            mod #module_name {
                #public_struct
                #raw_enum
                #data_enum

                #convert_impls
            }
        )
    };

    ret
}

fn attr_args(attr: TokenStream2) -> HashMap<String, TokenTree> {
    attr.into_iter()
        .group_by(|tk| !matches!(tk, TokenTree::Punct(p) if p.as_char() == ','))
        .into_iter()
        .filter_map(|(cond, c)| cond.then(|| c))
        .map(|mut triple| {
            let ident = triple.next();
            let eq_sign = triple.next();
            let value = triple.next();

            if !matches!(eq_sign, Some(TokenTree::Punct(eq_sign)) if eq_sign.as_char() == '=') {
                panic!(r#"Attribute arguments should be in the form of `key = value`"#)
            }

            match (ident, value) {
                (Some(TokenTree::Ident(ident)), Some(value)) => (ident.to_string(), value),
                _ => panic!(r#"Attribute arguments should be in the form of `key = "value"`"#),
            }
        })
        .collect()
}

#[cfg(test)]
mod test {
    use crate::hybrid_tagged_impl;
    use quote::quote;

    #[test]
    fn test_hybrid_tagged_impl() {
        let macro_out = hybrid_tagged_impl(
            quote!(tag = "type", fields = {frame: Number, slack: Slack,}),
            quote!(
                #[Derive(Serialize, Deserialize)]
                #[serde(some_other_thing)]
                pub(super) enum Variations {
                    A { task: T, time: U },
                    B { hours: H, intervals: I },
                    C,
                    // D(Wrong)
                }
            ),
        );

        println!("{}", macro_out)
    }
}
