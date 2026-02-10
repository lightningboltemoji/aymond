use crate::definition::ItemDefinition;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

pub fn create_query_builder(item: &ItemDefinition) -> TokenStream {
    let query_struct = format_ident!("{}Query", &item.name);
    let sort_key_struct = format_ident!("{}QuerySortKey", &item.name);

    let hash_key_ident = &item.hash_key.ident;
    let hash_key_typ = &item.sort_key.as_ref().unwrap().typ_ident;

    let mut chunks: Vec<TokenStream> = vec![];
    chunks.push(quote! {
        struct #query_struct {
            hk: Option<String>,
            sk: Option<String>,
        }
    });

    if item.sort_key.is_some() {
        chunks.push(quote! {
            impl #query_struct {
                fn new() -> Self {
                    Self { hk: None, sk: None }
                }

                fn #hash_key_ident (mut self, v: impl Into<#hash_key_typ>) -> #sort_key_struct {
                    self.hk = Some(v.into());
                    #sort_key_struct { q: self }
                }
            }

            struct #sort_key_struct {
                q: #query_struct,
            }
        });

        let sort_key_ident = &item.sort_key.as_ref().unwrap().ident;
        let sort_key_typ = &item.sort_key.as_ref().unwrap().typ_ident;
        let gt = format_ident!("{}_gt", sort_key_ident);
        chunks.push(quote! {
            impl #sort_key_struct {
                fn #gt (mut self, v: impl Into<#sort_key_typ>) -> #query_struct {
                    self.q.sk = Some(v.into());
                    self.q
                }
            }
        });
    } else {
        chunks.push(quote! {
            struct #query_struct {
                hk: Option<String>,
                sk: Option<String>,
            }

            impl #query_struct {
                fn new() -> Self {
                    Self { hk: None, sk: None }
                }

                fn #hash_key_ident (mut self, v: impl Into<#hash_key_typ>) -> #query_struct {
                    self.hk = Some(v.into());
                    self
                }
            }
        });
    }

    quote! {
        #( #chunks )*
    }
}
