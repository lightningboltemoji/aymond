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

    quote! {
        pub struct #ident;

        impl #ident {
            #( #accessors )*
        }
    }
}
