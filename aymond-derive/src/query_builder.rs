use crate::definition::ItemDefinition;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, parse_quote};

pub fn create_query_builder(item: &ItemDefinition) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let query_struct = format_ident!("{}Query", &item.name);
    let hash_key_struct = format_ident!("{}QueryHashKey", &item.name);

    let hash_key_attr_name = &item.hash_key.attr_name;
    let hash_key_ident = &item.hash_key.ident;
    let hash_key_typ = &item.hash_key.typ_ident;
    let hash_key_boxer = &item.hash_key.key_boxer_for(&parse_quote!(self.hk.unwrap()));

    let mut chunks: Vec<TokenStream> = vec![];

    if item.sort_key.is_some() {
        let sort_key_struct = format_ident!("{}QuerySortKey", &item.name);
        let sort_key_ident = &item.sort_key.as_ref().unwrap().ident;
        let sort_key_attr_name = &item.sort_key.as_ref().unwrap().attr_name;
        let sort_key_typ = &item.sort_key.as_ref().unwrap().typ_ident;

        let sort_key = item.sort_key.as_ref().unwrap();
        let sort_key_b_boxer = sort_key.key_boxer_for(&parse_quote!(self.b.unwrap()));
        let sort_key_c_boxer = sort_key.key_boxer_for(&parse_quote!(self.c.unwrap()));

        chunks.push(quote! {
            struct #hash_key_struct {
                q: #query_struct,
            }

            struct #query_struct {
                hk: Option<String>,
                qs: Option<String>,
                b: Option<#sort_key_typ>,
                c: Option<#sort_key_typ>,
            }

            impl #query_struct {
                fn new() -> #hash_key_struct {
                    let q = #query_struct { hk: None, qs: None, b: None, c: None };
                    #hash_key_struct { q }
                }
            }

            impl #hash_key_struct {
                fn #hash_key_ident (mut self, v: impl Into<#hash_key_typ>) -> #sort_key_struct {
                    self.q.hk = Some(v.into());
                    #sort_key_struct { q: self.q }
                }
            }

            struct #sort_key_struct {
                q: #query_struct,
            }

            impl Into<(
                String,
                ::std::collections::HashMap<String, String>,
                ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>,
            )> for #query_struct {
                fn into(self) -> (
                    String,
                    ::std::collections::HashMap<String, String>,
                    ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>,
                ) {
                    let exp = self.qs.unwrap();

                    let mut key_names = ::std::collections::HashMap::new();
                    key_names.insert("#hk".to_string(), #hash_key_attr_name.to_string());
                    key_names.insert("#sk".to_string(), #sort_key_attr_name.to_string());

                    let mut key_values = ::std::collections::HashMap::new();
                    key_values.insert(":hk".to_string(), #hash_key_boxer);
                    if self.b.is_some() {
                        key_values.insert(":b".to_string(), #sort_key_b_boxer);
                    }
                    if self.c.is_some() {
                        key_values.insert(":c".to_string(), #sort_key_c_boxer);
                    }

                    (exp, key_names, key_values)
                }
            }
        });

        let mut comparisons = vec![
            (
                format_ident!("{}_gt", sort_key_ident),
                "#hk = :hk AND #sk > :b",
                vec![quote! {b}],
            ),
            (
                format_ident!("{}_ge", sort_key_ident),
                "#hk = :hk AND #sk >= :b",
                vec![quote! {b}],
            ),
            (
                format_ident!("{}_lt", sort_key_ident),
                "#hk = :hk AND #sk < :b",
                vec![quote! {b}],
            ),
            (
                format_ident!("{}_le", sort_key_ident),
                "#hk = :hk AND #sk <= :b",
                vec![quote! {b}],
            ),
            (
                format_ident!("{}_between", sort_key_ident),
                "#hk = :hk AND #sk BETWEEN :b AND :c",
                vec![quote! {b}, quote! {c}],
            ),
        ];
        if hash_key_typ == "String" {
            comparisons.push((
                format_ident!("{}_begins_with", sort_key_ident),
                "#hk = :hk AND begins_with(#sk, :b)",
                vec![quote! {b}],
            ));
        }

        for (fn_name, key_expression, vars) in comparisons {
            chunks.push(quote! {
                impl #sort_key_struct {
                    fn #fn_name (mut self, #( #vars: impl Into<#sort_key_typ>, )*) -> #query_struct {
                        self.q.qs = Some(#key_expression.into());
                        #( self.q.#vars = Some(#vars.into()); )*
                        self.q
                    }
                }
            })
        }
    } else {
        chunks.push(quote! {
            struct #hash_key_struct {
                q: #query_struct,
            }

            struct #query_struct {
                hk: Option<String>,
                qs: Option<String>,
            }

            impl #query_struct {
                fn new() -> #hash_key_struct {
                    let q = #query_struct { hk: None, qs: None };
                    #hash_key_struct { q }
                }
            }

            impl #hash_key_struct {
                fn #hash_key_ident (mut self, v: impl Into<#hash_key_typ>) -> #query_struct {
                    self.q.hk = Some(v.into());
                    self.q.qs = Some("#hk = :hk".into());
                    self.q
                }
            }

            impl Into<(
                String,
                ::std::collections::HashMap<String, String>,
                ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>,
            )> for #query_struct {
                fn into(self) -> (
                    String,
                    ::std::collections::HashMap<String, String>,
                    ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>,
                ) {
                    let exp = self.qs.unwrap();

                    let mut key_names = ::std::collections::HashMap::new();
                    key_names.insert("#hk".to_string(), #hash_key_attr_name.to_string());

                    let mut key_values = ::std::collections::HashMap::new();
                    key_values.insert(":hk".to_string(), #hash_key_boxer);

                    (exp, key_names, key_values)
                }
            }
        });
    }

    quote! {
        #( #chunks )*
    }
}
