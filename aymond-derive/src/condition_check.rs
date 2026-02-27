use crate::definition::ItemDefinition;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, parse_quote};

pub fn create_condition_check_builder(item: &ItemDefinition) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let table_struct = format_ident!("{}Table", &item.name);
    let condition_check_struct = format_ident!("{}ConditionCheck", &item.name);
    let hash_key_struct = format_ident!("{}ConditionCheckHashKey", &item.name);
    let condition_builder_struct = format_ident!("{}Condition", &item.name);

    let hash_key = item.hash_key.as_ref().unwrap();
    let hash_key_attr_name = &hash_key.ddb_name;
    let hash_key_ident = &hash_key.field;
    let hash_key_typ = &hash_key.ty;
    let hash_key_boxer = &hash_key.to_attribute_value(&parse_quote!(hk));

    let (builders, build_key_map) = if item.sort_key.is_some() {
        let sort_key_struct = format_ident!("{}ConditionCheckSortKey", &item.name);
        let sort_key = item.sort_key.as_ref().unwrap();
        let sort_key_ident = &sort_key.field;
        let sort_key_attr_name = &sort_key.ddb_name;
        let sort_key_typ = &sort_key.ty;
        let sort_key_boxer = &sort_key.to_attribute_value(&parse_quote!(sk));

        let build_key_map = quote! {
            let hk = self.hk.unwrap();
            let sk = self.sk.unwrap();
            let mut key_values = ::std::collections::HashMap::new();
            key_values.insert(#hash_key_attr_name.to_string(), #hash_key_boxer);
            key_values.insert(#sort_key_attr_name.to_string(), #sort_key_boxer);
        };

        let builders = quote! {
            pub struct #hash_key_struct<'a> {
                q: #condition_check_struct<'a>,
            }

            pub struct #condition_check_struct<'a> {
                table: &'a #table_struct,
                hk: Option<#hash_key_typ>,
                sk: Option<#sort_key_typ>,
                cond: #condition_builder_struct,
            }

            impl<'a> #condition_check_struct<'a> {
                fn new(table: &'a #table_struct) -> #hash_key_struct<'a> {
                    let q = #condition_check_struct { table, hk: None, sk: None, cond: #condition_builder_struct::new() };
                    #hash_key_struct { q }
                }
            }

            impl<'a> #hash_key_struct<'a> {
                pub fn #hash_key_ident (mut self, v: impl Into<#hash_key_typ>) -> #sort_key_struct<'a> {
                    self.q.hk = Some(v.into());
                    #sort_key_struct { q: self.q }
                }
            }

            pub struct #sort_key_struct<'a> {
                q: #condition_check_struct<'a>,
            }

            impl<'a> #sort_key_struct<'a> {
                pub fn #sort_key_ident (mut self, sk: impl Into<#sort_key_typ>) -> #condition_check_struct<'a> {
                    self.q.sk = Some(sk.into());
                    self.q
                }
            }
        };
        (builders, build_key_map)
    } else {
        let build_key_map = quote! {
            let hk = self.hk.unwrap();
            let mut key_values = ::std::collections::HashMap::new();
            key_values.insert(#hash_key_attr_name.to_string(), #hash_key_boxer);
        };

        let builders = quote! {
            pub struct #hash_key_struct<'a> {
                q: #condition_check_struct<'a>,
            }

            pub struct #condition_check_struct<'a> {
                table: &'a #table_struct,
                hk: Option<#hash_key_typ>,
                cond: #condition_builder_struct,
            }

            impl<'a> #condition_check_struct<'a> {
                fn new(table: &'a #table_struct) -> #hash_key_struct<'a> {
                    let q = #condition_check_struct { table, hk: None, cond: #condition_builder_struct::new() };
                    #hash_key_struct { q }
                }
            }

            impl<'a> #hash_key_struct<'a> {
                pub fn #hash_key_ident (mut self, v: impl Into<#hash_key_typ>) -> #condition_check_struct<'a> {
                    self.q.hk = Some(v.into());
                    self.q
                }
            }
        };
        (builders, build_key_map)
    };

    quote! {
        #builders

        impl<'a> #condition_check_struct<'a> {
            pub fn condition<F, R>(mut self, f: F) -> #condition_check_struct<'a>
            where
                F: FnOnce(&#condition_builder_struct) -> R,
                R: ::aymond::condition::IntoOptionalCondExpr,
            {
                let result = f(&self.cond);
                if let Some(expr) = result.into_optional_cond_expr() {
                    self.cond.set_expr(expr);
                }
                self
            }
        }

        impl<'a> Into<#aws_sdk_dynamodb::types::ConditionCheck> for #condition_check_struct<'a> {
            fn into(self) -> #aws_sdk_dynamodb::types::ConditionCheck {
                let (cond_expr, expr_name, expr_value) = self.cond.build();
                let table_name = &self.table.table_name;
                #build_key_map
                let mut b = #aws_sdk_dynamodb::types::ConditionCheck::builder()
                    .table_name(table_name)
                    .set_key(Some(key_values));
                if cond_expr.is_some() {
                    b = b.set_condition_expression(cond_expr)
                        .set_expression_attribute_names(expr_name)
                        .set_expression_attribute_values(expr_value);
                }
                b.build().unwrap()
            }
        }
    }
}
