use crate::ItemDefinition;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub fn create_condition_builder(item: &ItemDefinition) -> TokenStream {
    let ident = format_ident!("{}Condition", &item.name);
    let fields_ident = format_ident!("{}ConditionFields", &item.name);

    let hash_key_ddb_name = &item.hash_key.as_ref().unwrap().ddb_name;

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
                versioning: std::cell::Cell<bool>,
                version_value: std::cell::Cell<Option<#ver_ty>>,
                existence: std::cell::Cell<::aymond::condition::ExistenceCheck>,
                expr: Option<::aymond::condition::CondExpr>,
            }

            pub struct #fields_ident;

            impl ::aymond::condition::IntoOptionalCondExpr for #fields_ident {
                fn into_optional_cond_expr(self) -> Option<::aymond::condition::CondExpr> {
                    None
                }
            }

            impl #ident {
                fn new() -> Self {
                    Self {
                        versioning: std::cell::Cell::new(true),
                        version_value: std::cell::Cell::new(None),
                        existence: std::cell::Cell::new(::aymond::condition::ExistenceCheck::None),
                        expr: None,
                    }
                }

                pub fn disable_versioning(&self) -> #fields_ident {
                    self.versioning.set(false);
                    #fields_ident
                }

                pub fn is_versioning_enabled(&self) -> bool {
                    self.versioning.get()
                }

                pub fn must_exist(&self) -> #fields_ident {
                    self.existence.set(::aymond::condition::ExistenceCheck::MustExist);
                    #fields_ident
                }

                pub fn must_not_exist(&self) -> #fields_ident {
                    self.existence.set(::aymond::condition::ExistenceCheck::MustNotExist);
                    #fields_ident
                }

                fn set_version_value(&self, v: #ver_ty) {
                    self.version_value.set(Some(v));
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
                    let existence = self.existence.get();
                    let versioning = self.versioning.get();
                    let version_value = self.version_value.get();

                    // Priority: explicit existence > version-zero auto > normal versioning
                    let version_expr = match existence {
                        ::aymond::condition::ExistenceCheck::MustNotExist => {
                            Some(::aymond::condition::CondExpr::AttributeNotExists {
                                path: vec![::aymond::condition::PathSegment::Attr(#hash_key_ddb_name.to_string())],
                            })
                        }
                        ::aymond::condition::ExistenceCheck::MustExist => {
                            Some(::aymond::condition::CondExpr::AttributeExists {
                                path: vec![::aymond::condition::PathSegment::Attr(#hash_key_ddb_name.to_string())],
                            })
                        }
                        ::aymond::condition::ExistenceCheck::None => {
                            if versioning {
                                match version_value {
                                    Some(v) if v == 0 => {
                                        // Version zero: item must not exist yet
                                        Some(::aymond::condition::CondExpr::AttributeNotExists {
                                            path: vec![::aymond::condition::PathSegment::Attr(#hash_key_ddb_name.to_string())],
                                        })
                                    }
                                    Some(v) => {
                                        Some(::aymond::condition::CondExpr::Comparison {
                                            path: vec![::aymond::condition::PathSegment::Attr(#ver_ddb_name.to_string())],
                                            op: "=",
                                            value: ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue::N(
                                                v.to_string()
                                            ),
                                        })
                                    }
                                    None => None,
                                }
                            } else {
                                None
                            }
                        }
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
                            (
                                Some(e),
                                if n.is_empty() { None } else { Some(n) },
                                if v.is_empty() { None } else { Some(v) },
                            )
                        }
                        None => (None, None, None),
                    }
                }
            }

            impl #fields_ident {
                #( #accessors )*
            }
        }
    } else {
        quote! {
            pub struct #ident {
                existence: std::cell::Cell<::aymond::condition::ExistenceCheck>,
                expr: Option<::aymond::condition::CondExpr>,
            }

            pub struct #fields_ident;

            impl ::aymond::condition::IntoOptionalCondExpr for #fields_ident {
                fn into_optional_cond_expr(self) -> Option<::aymond::condition::CondExpr> {
                    None
                }
            }

            impl #ident {
                fn new() -> Self {
                    Self {
                        existence: std::cell::Cell::new(::aymond::condition::ExistenceCheck::None),
                        expr: None,
                    }
                }

                pub fn must_exist(&self) -> #fields_ident {
                    self.existence.set(::aymond::condition::ExistenceCheck::MustExist);
                    #fields_ident
                }

                pub fn must_not_exist(&self) -> #fields_ident {
                    self.existence.set(::aymond::condition::ExistenceCheck::MustNotExist);
                    #fields_ident
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
                    let existence = self.existence.get();

                    let existence_expr = match existence {
                        ::aymond::condition::ExistenceCheck::MustNotExist => {
                            Some(::aymond::condition::CondExpr::AttributeNotExists {
                                path: vec![::aymond::condition::PathSegment::Attr(#hash_key_ddb_name.to_string())],
                            })
                        }
                        ::aymond::condition::ExistenceCheck::MustExist => {
                            Some(::aymond::condition::CondExpr::AttributeExists {
                                path: vec![::aymond::condition::PathSegment::Attr(#hash_key_ddb_name.to_string())],
                            })
                        }
                        ::aymond::condition::ExistenceCheck::None => None,
                    };

                    let combined = match (self.expr, existence_expr) {
                        (Some(u), Some(v)) => Some(u.and(v)),
                        (Some(u), None) => Some(u),
                        (None, Some(v)) => Some(v),
                        (None, None) => None,
                    };

                    match combined {
                        Some(expr) => {
                            let (e, n, v) = expr.build();
                            (
                                Some(e),
                                if n.is_empty() { None } else { Some(n) },
                                if v.is_empty() { None } else { Some(v) },
                            )
                        }
                        None => (None, None, None),
                    }
                }
            }

            impl #fields_ident {
                #( #accessors )*
            }
        }
    }
}
