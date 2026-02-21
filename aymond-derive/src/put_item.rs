use crate::definition::ItemDefinition;
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
                F: FnOnce(#condition_builder_struct) -> ::aymond::condition::CondExpr
            {
                let (cond_expr, expr_name, expr_value) = f(#condition_builder_struct).build();
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

        impl<'a> Into<#aws_sdk_dynamodb::types::Put> for #put_item_struct<'a> {
            fn into(self) -> #aws_sdk_dynamodb::types::Put {
                let mut b = #aws_sdk_dynamodb::types::Put::builder()
                    .table_name(&self.table.table_name)
                    .set_item(Some(self.i.expect("item not set").into()));
                if self.cond_expr.is_some() {
                    b = b.set_condition_expression(self.cond_expr)
                        .set_expression_attribute_names(self.expr_name)
                        .set_expression_attribute_values(self.expr_value);
                }
                b.build().unwrap()
            }
        }
    }
}

fn create_condition_builder(item: &ItemDefinition) -> (TokenStream, Ident) {
    let ident = format_ident!("{}Condition", &item.name);

    let accessors: Vec<TokenStream> = item
        .all_attributes()
        .map(|attr| {
            let fn_name = &attr.field;
            let ddb_name = &attr.ddb_name;
            let return_type = attr.condition_path_type();
            quote! {
                pub fn #fn_name(&self) -> #return_type {
                    ::aymond::condition::ConditionPathRoot::with_prefix(
                        vec![::aymond::condition::PathSegment::Attr(#ddb_name.to_string())]
                    )
                }
            }
        })
        .collect();

    let imp = quote! {
        pub struct #ident;

        impl #ident {
            #( #accessors )*
        }
    };
    (imp, ident)
}
