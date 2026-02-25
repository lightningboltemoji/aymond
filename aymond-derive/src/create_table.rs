use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, parse_quote};

use crate::definition::{GsiDefinition, ItemAttribute, ItemDefinition, LsiDefinition};

pub fn create_create_method(def: &ItemDefinition) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);
    let name = format_ident!("{}", &def.name);

    // Step 1: Collect all unique attribute definitions, deduped by ddb_name.
    // Order: primary HK, primary SK, then GSIs (sorted by name), then LSIs (sorted by name).
    let mut candidates: Vec<&ItemAttribute> = Vec::new();

    if let Some(hk) = &def.hash_key {
        candidates.push(hk);
    }
    if let Some(sk) = &def.sort_key {
        candidates.push(sk);
    }

    let mut gsi_defs: Vec<&GsiDefinition> = def.global_secondary_indexes.values().collect();
    gsi_defs.sort_by_key(|g| &g.name);

    for gsi in &gsi_defs {
        if let Some(hk) = &gsi.hash_key {
            candidates.push(hk);
        }
        if let Some(sk) = &gsi.sort_key {
            candidates.push(sk);
        }
    }

    let mut lsi_defs: Vec<&LsiDefinition> = def.local_secondary_indexes.values().collect();
    lsi_defs.sort_by_key(|l| &l.name);

    for lsi in &lsi_defs {
        candidates.push(&lsi.sort_key);
    }

    // Dedup by ddb_name (first seen wins).
    let mut seen: Vec<String> = Vec::new();
    let mut attr_names: Vec<String> = Vec::new();
    let mut attr_scalar_types: Vec<Expr> = Vec::new();

    for attr in candidates {
        if !seen.contains(&attr.ddb_name) {
            seen.push(attr.ddb_name.clone());
            attr_names.push(attr.ddb_name.clone());
            attr_scalar_types.push(attr.scalar_type());
        }
    }

    // Step 2: Build GSI tokens.
    let gsi_tokens: Vec<TokenStream> = gsi_defs
        .iter()
        .map(|gsi| {
            let index_name = &gsi.name;
            let mut key_schema_calls: Vec<TokenStream> = Vec::new();

            if let Some(hk) = &gsi.hash_key {
                let hk_name = &hk.ddb_name;
                key_schema_calls.push(quote! {
                    .key_schema(
                        #aws_sdk_dynamodb::types::KeySchemaElement::builder()
                            .attribute_name(#hk_name)
                            .key_type(#aws_sdk_dynamodb::types::KeyType::Hash)
                            .build()
                            .unwrap()
                    )
                });
            }

            if let Some(sk) = &gsi.sort_key {
                let sk_name = &sk.ddb_name;
                key_schema_calls.push(quote! {
                    .key_schema(
                        #aws_sdk_dynamodb::types::KeySchemaElement::builder()
                            .attribute_name(#sk_name)
                            .key_type(#aws_sdk_dynamodb::types::KeyType::Range)
                            .build()
                            .unwrap()
                    )
                });
            }

            quote! {
                #aws_sdk_dynamodb::types::GlobalSecondaryIndex::builder()
                    .index_name(#index_name)
                    #(#key_schema_calls)*
                    .projection(
                        #aws_sdk_dynamodb::types::Projection::builder()
                            .projection_type(#aws_sdk_dynamodb::types::ProjectionType::All)
                            .build()
                    )
                    .build()
                    .unwrap()
            }
        })
        .collect();

    // Step 3: Build LSI tokens.
    let table_hk_name = def.hash_key.as_ref().unwrap().ddb_name.clone();
    let lsi_tokens: Vec<TokenStream> = lsi_defs
        .iter()
        .map(|lsi| {
            let index_name = &lsi.name;
            let sk_name = &lsi.sort_key.ddb_name;

            quote! {
                #aws_sdk_dynamodb::types::LocalSecondaryIndex::builder()
                    .index_name(#index_name)
                    .key_schema(
                        #aws_sdk_dynamodb::types::KeySchemaElement::builder()
                            .attribute_name(#table_hk_name)
                            .key_type(#aws_sdk_dynamodb::types::KeyType::Hash)
                            .build()
                            .unwrap()
                    )
                    .key_schema(
                        #aws_sdk_dynamodb::types::KeySchemaElement::builder()
                            .attribute_name(#sk_name)
                            .key_type(#aws_sdk_dynamodb::types::KeyType::Range)
                            .build()
                            .unwrap()
                    )
                    .projection(
                        #aws_sdk_dynamodb::types::Projection::builder()
                            .projection_type(#aws_sdk_dynamodb::types::ProjectionType::All)
                            .build()
                    )
                    .build()
                    .unwrap()
            }
        })
        .collect();

    // Step 4: Assemble the method.
    let gsi_call = if gsi_tokens.is_empty() {
        quote! {}
    } else {
        quote! { .set_global_secondary_indexes(Some(vec![#(#gsi_tokens),*])) }
    };
    let lsi_call = if lsi_tokens.is_empty() {
        quote! {}
    } else {
        quote! { .set_local_secondary_indexes(Some(vec![#(#lsi_tokens),*])) }
    };

    quote! {
        async fn create(&self, err_if_exists: bool) -> Result<
            (), #aws_sdk_dynamodb::error::SdkError<
                #aws_sdk_dynamodb::operation::create_table::CreateTableError,
                #aws_sdk_dynamodb::config::http::HttpResponse
            >
        > {
            let res = self.client.create_table()
                .table_name(&self.table_name)
                .set_key_schema(Some(#name::key_schemas()))
                .set_attribute_definitions(Some(vec![
                    #(
                        #aws_sdk_dynamodb::types::AttributeDefinition::builder()
                            .attribute_name(#attr_names)
                            .attribute_type(#attr_scalar_types)
                            .build()
                            .unwrap()
                    ),*
                ]))
                #gsi_call
                #lsi_call
                .billing_mode(#aws_sdk_dynamodb::types::BillingMode::PayPerRequest)
                .send();
            match res.await {
                Err(e) => match e {
                    #aws_sdk_dynamodb::error::SdkError::ServiceError(ref context)
                        if !err_if_exists && context.err().is_resource_in_use_exception() => Ok(()),
                    _ => Err(e)
                }
                _ => Ok(())
            }
        }
    }
}
