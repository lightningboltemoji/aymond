use proc_macro2::TokenStream;
use quote::quote;
use syn::DeriveInput;

use crate::{ItemDefinition, marshal};

pub fn create_nested_item(input: &mut DeriveInput) -> TokenStream {
    let def = ItemDefinition::new(input, true);
    let from_into = marshal::from_into_item_structure(&def);
    quote! {
        #[derive(Debug, PartialEq)]
        #input
        #from_into
    }
}
