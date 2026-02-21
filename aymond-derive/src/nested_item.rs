use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::DeriveInput;

use crate::{ItemDefinition, marshal};

pub fn create_nested_item(input: &mut DeriveInput) -> TokenStream {
    let def = ItemDefinition::new(input, true);
    let from_into = marshal::from_into_item_structure(&def);
    let condition_path = create_condition_path(&def);
    quote! {
        #[derive(Debug, PartialEq)]
        #input
        #from_into
        #condition_path
    }
}

pub fn create_condition_path(def: &ItemDefinition) -> TokenStream {
    let path_struct = format_ident!("{}ConditionPath", &def.name);

    let accessors: Vec<TokenStream> = def
        .all_attributes()
        .map(|attr| {
            let fn_name = &attr.field;
            let ddb_name = &attr.ddb_name;
            let return_type = attr.condition_path_type();
            quote! {
                pub fn #fn_name(&self) -> #return_type {
                    let mut path = self.path_prefix.clone();
                    path.push(::aymond::condition::PathSegment::Attr(#ddb_name.to_string()));
                    ::aymond::condition::ConditionPathRoot::with_prefix(path)
                }
            }
        })
        .collect();

    quote! {
        pub struct #path_struct {
            path_prefix: Vec<::aymond::condition::PathSegment>,
        }

        impl ::aymond::condition::ConditionPathRoot for #path_struct {
            fn with_prefix(path: Vec<::aymond::condition::PathSegment>) -> Self {
                Self { path_prefix: path }
            }
        }

        impl #path_struct {
            #( #accessors )*
        }
    }
}
