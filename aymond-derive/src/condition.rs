use crate::ItemDefinition;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub fn create_condition_builder(item: &ItemDefinition) -> TokenStream {
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

    if let Some(ver_attr) = &item.version_attribute {
        let ver_ty = &ver_attr.ty;
        let ver_ddb_name = &ver_attr.ddb_name;

        quote! {
            pub struct #ident {
                versioning: bool,
                version_value: Option<#ver_ty>,
                expr: Option<::aymond::condition::CondExpr>,
            }

            impl #ident {
                fn new() -> Self {
                    Self { versioning: true, version_value: None, expr: None }
                }

                pub fn enable_versioning(&mut self) -> &mut Self {
                    self.versioning = true;
                    self
                }

                pub fn disable_versioning(&mut self) -> &mut Self {
                    self.versioning = false;
                    self
                }

                fn set_version_value(&mut self, v: #ver_ty) {
                    self.version_value = Some(v);
                }

                fn set_expr(&mut self, expr: ::aymond::condition::CondExpr) {
                    self.expr = Some(expr);
                }

                #( #accessors )*

                fn build(
                    self,
                ) -> (
                    Option<String>,
                    Option<::std::collections::HashMap<String, String>>,
                    Option<::std::collections::HashMap<String, ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue>>,
                ) {
                    let version_expr = if self.versioning && self.version_value.is_some() {
                        Some(::aymond::condition::CondExpr::Comparison {
                            path: vec![::aymond::condition::PathSegment::Attr(#ver_ddb_name.to_string())],
                            op: "=",
                            value: ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue::N(
                                self.version_value.unwrap().to_string()
                            ),
                        })
                    } else {
                        None
                    };

                    let combined = match (self.expr, version_expr) {
                        (Some(u), Some(v)) => Some(u.and(v)),
                        (Some(u), None) => Some(u),
                        (None, Some(v)) => Some(v),
                        (None, None) => None,
                    };

                    match combined {
                        Some(expr) => {
                            let (e, n, v) = expr.build();
                            (Some(e), Some(n), Some(v))
                        }
                        None => (None, None, None),
                    }
                }
            }
        }
    } else {
        quote! {
            pub struct #ident {
                expr: Option<::aymond::condition::CondExpr>,
            }

            impl #ident {
                fn new() -> Self {
                    Self { expr: None }
                }

                fn set_expr(&mut self, expr: ::aymond::condition::CondExpr) {
                    self.expr = Some(expr);
                }

                #( #accessors )*

                fn build(
                    self,
                ) -> (
                    Option<String>,
                    Option<::std::collections::HashMap<String, String>>,
                    Option<::std::collections::HashMap<String, ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue>>,
                ) {
                    match self.expr {
                        Some(expr) => {
                            let (e, n, v) = expr.build();
                            (Some(e), Some(n), Some(v))
                        }
                        None => (None, None, None),
                    }
                }
            }
        }
    }
}
