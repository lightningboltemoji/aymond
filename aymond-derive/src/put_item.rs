use crate::{ItemAttribute, definition::ItemDefinition};
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{Expr, parse_quote};

pub fn create_put_item_builder(item: &ItemDefinition) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let item_struct = format_ident!("{}", &item.name);
    let table_struct = format_ident!("{}Table", &item.name);
    let put_item_struct = format_ident!("{}PutItem", &item.name);

    let (condition_builder, condition_builder_struct) = create_condition_builder(item);
    quote! {
        #condition_builder

        struct #put_item_struct<'a> {
            table: &'a #table_struct,
            i: Option<#item_struct>,
            cond_expr: Option<String>,
            expr_name: Option<::std::collections::HashMap<String, String>>,
            expr_value: Option<::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>>,
        }

        impl<'a> #put_item_struct<'a> {
            fn new(table: &'a #table_struct) -> #put_item_struct<'a> {
                #put_item_struct { table, i: None, cond_expr: None, expr_name: None, expr_value: None }
            }
        }

        impl<'a> #put_item_struct<'a> {
            fn item(mut self, v: #item_struct) -> #put_item_struct<'a> {
                self.i = Some(v);
                self
            }

            fn condition<F>(mut self, f: F) -> #put_item_struct<'a>
            where
                F: FnOnce(#condition_builder_struct) -> #condition_builder_struct
            {
                let (cond_expr, expr_name, expr_value) = f(#condition_builder_struct::new()).into();
                self.cond_expr = Some(cond_expr);
                self.expr_name = Some(expr_name);
                self.expr_value = Some(expr_value);
                self
            }

            async fn raw<F>(
                self,
                f: F,
            ) -> Result<
                ::aymond::shim::aws_sdk_dynamodb::operation::put_item::PutItemOutput,
                ::aymond::shim::aws_sdk_dynamodb::error::SdkError<
                    ::aymond::shim::aws_sdk_dynamodb::operation::put_item::PutItemError,
                    ::aymond::shim::aws_sdk_dynamodb::config::http::HttpResponse,
                >,
            >
            where
                F: FnOnce(#aws_sdk_dynamodb::operation::put_item::builders::PutItemFluentBuilder)
                -> #aws_sdk_dynamodb::operation::put_item::builders::PutItemFluentBuilder
            {
                let mut req = f(self.table.client.put_item())
                    .table_name(&self.table.table_name)
                    .set_item(Some(self.i.expect("item not set").into()));
                if self.cond_expr.is_some() {
                    req = req.set_condition_expression(self.cond_expr)
                        .set_expression_attribute_names(self.expr_name)
                        .set_expression_attribute_values(self.expr_value);
                }
                req
                    .send()
                    .await
            }

            async fn send(self) -> Result<
                (),
                #aws_sdk_dynamodb::error::SdkError<
                    #aws_sdk_dynamodb::operation::put_item::PutItemError,
                    #aws_sdk_dynamodb::config::http::HttpResponse
                >
            > {
                self.raw(|r| r).await?;
                Ok(())
            }
        }
    }
}

fn create_condition_builder(item: &ItemDefinition) -> (TokenStream, Ident) {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let ident = format_ident!("{}PutItemCondition", &item.name);

    let mut statics: Vec<TokenStream> = vec![];
    for (fn_name, val) in [
        ("and", "AND"),
        ("or", "OR"),
        ("not", "NOT"),
        ("paren_open", "("),
        ("paren_close", ")"),
    ] {
        let fn_name = format_ident!("{}", fn_name);
        statics.push(quote! {
            fn #fn_name(mut self) -> Self {
                self.fragments.push(#val.into());
                self
            }
        });
    }

    let attribute_ops: Vec<TokenStream> = item
        .all_attributes()
        .map(|i: &ItemAttribute| {
            let attr_name = &i.ddb_name;
            let attr_typ = &i.ty_value;
            let boxer = if i.is_option {
                i.to_attribute_value(&parse_quote!(Some(v)))
            } else {
                i.to_attribute_value(&parse_quote!(v))
            };
            let fn_name = format_ident!("{}_eq", &i.field);
            quote! {
                fn #fn_name(mut self, v: #attr_typ) -> Self {
                    let id = self.cur;
                    self.cur = (id as u8 + 1) as char;
                    self.fragments.push(format!("#{0} = :{0}", id));
                    self.expr_name.insert(format!("#{}", id), #attr_name.to_string());
                    self.expr_value.insert(format!(":{}", id), #boxer);
                    self
                }
            }
        })
        .collect();

    let imp = quote! {
        struct #ident {
            fragments: Vec<String>,
            expr_name: ::std::collections::HashMap<String, String>,
            expr_value: ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>,
            cur: char,
        }

        impl #ident {
            fn new() -> Self {
                Self {
                    fragments: vec![],
                    expr_name: ::std::collections::HashMap::new(),
                    expr_value: ::std::collections::HashMap::new(),
                    cur: 'a',
                }
            }

            #( #statics )*
            #( #attribute_ops )*
        }

        impl Into<(
            String,
            ::std::collections::HashMap<String, String>,
            ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>
        )> for #ident {
            fn into(self) -> (
                String,
                ::std::collections::HashMap<String, String>,
                ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>
            ) {
                (self.fragments.join(" "), self.expr_name, self.expr_value)
            }
        }
    };
    (imp, ident)
}
