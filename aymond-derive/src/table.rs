use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Expr, parse_quote};

use crate::{
    ItemDefinition,
    batch_get_item::create_batch_get_builder,
    batch_write_item::create_batch_write_builder,
    condition::create_condition_builder,
    condition_check::create_condition_check_builder,
    create_scan_builder,
    create_table::create_create_method,
    delete_item::create_delete_builder,
    get_item::create_get_builder,
    put_item::create_put_item_builder,
    query::{create_index_query_builders, create_main_query_builder},
    update_item::create_update_builder,
};

pub fn create_table(item: &ItemDefinition) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let name = format_ident!("{}", &item.name);
    let table_struct = format_ident!("{}Table", &name);
    let get_item_struct = format_ident!("{}GetItem", &name);
    let get_item_hash_key_struct = format_ident!("{}GetItemHashKey", &name);
    let put_item_struct = format_ident!("{}PutItem", &name);
    let update_item_struct = format_ident!("{}UpdateItem", &name);
    let update_item_hash_key_struct = format_ident!("{}UpdateItemHashKey", &name);
    let query_struct = format_ident!("{}Query", &name);
    let query_hash_key_struct = format_ident!("{}QueryHashKey", &name);
    let scan_struct = format_ident!("{}Scan", &name);
    let batch_get_struct = format_ident!("{}BatchGetItem", &name);
    let delete_item_struct = format_ident!("{}DeleteItem", &name);
    let delete_item_hash_key_struct = format_ident!("{}DeleteItemHashKey", &name);
    let batch_write_struct = format_ident!("{}BatchWriteItem", &name);
    let condition_check_struct = format_ident!("{}ConditionCheck", &name);
    let condition_check_hash_key_struct = format_ident!("{}ConditionCheckHashKey", &name);

    let get_item = create_get_builder(item);
    let put_item = create_put_item_builder(item);
    let condition = create_condition_builder(item);
    let update_item = create_update_builder(item);
    let query = create_main_query_builder(item);
    let query_index = create_index_query_builders(item);
    let scan = create_scan_builder(item);
    let batch_get = create_batch_get_builder(item);
    let delete_item = create_delete_builder(item);
    let batch_write = create_batch_write_builder(item);
    let condition_check = create_condition_check_builder(item);
    let create_method = create_create_method(item);

    quote! {
        #get_item
        #put_item
        #update_item
        #condition
        #query
        #query_index
        #scan
        #batch_get
        #delete_item
        #batch_write
        #condition_check

        pub struct #table_struct {
            aymond: ::aymond::Aymond,
            table_name: String,
        }

        impl ::std::fmt::Debug for #table_struct {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                f.debug_struct(stringify!(#table_struct))
                    .field("aymond", &self.aymond)
                    .field("table_name", &self.table_name)
                    .finish()
            }
        }

        impl<'a> Table<'a, #name, #get_item_struct<'a>, #get_item_hash_key_struct<'a>, #put_item_struct<'a>, #update_item_struct<'a>, #update_item_hash_key_struct<'a>, #query_struct<'a>, #query_hash_key_struct<'a>, #scan_struct<'a>, #batch_get_struct<'a>, #delete_item_struct<'a>, #delete_item_hash_key_struct<'a>, #batch_write_struct<'a>, #condition_check_struct<'a>, #condition_check_hash_key_struct<'a>> for #table_struct {

            fn new(
                client: &'a ::aymond::Aymond,
                table_name: impl ::core::convert::Into<String>,
            ) -> Self {
                Self {
                    table_name: table_name.into(),
                    aymond: client.clone(),
                }
            }

            #create_method

            async fn delete(&self, err_if_not_exists: bool) -> Result<
                (), #aws_sdk_dynamodb::error::SdkError<
                    #aws_sdk_dynamodb::operation::delete_table::DeleteTableError,
                    #aws_sdk_dynamodb::config::http::HttpResponse
                >
            > {
                let res = self.aymond.client.delete_table()
                    .table_name(&self.table_name)
                    .send();
                match res.await {
                    Err(e) => match e {
                        #aws_sdk_dynamodb::error::SdkError::ServiceError(ref context)
                            if !err_if_not_exists && context.err().is_resource_not_found_exception() => Ok(()),
                        _ => Err(e)
                    }
                    _ => Ok(())
                }
            }

            fn get(&'a self) -> #get_item_hash_key_struct<'a> {
                #get_item_struct::new(self)
            }

            fn put(&'a self) -> #put_item_struct<'a> {
                #put_item_struct::new(self)
            }

            fn update(&'a self) -> #update_item_hash_key_struct<'a> {
                #update_item_struct::new(self)
            }

            fn query(&'a self) -> #query_hash_key_struct<'a> {
                #query_struct::new(self)
            }

            fn scan(&'a self) -> #scan_struct<'a> {
                #scan_struct::new(self)
            }

            fn batch_get(&'a self) -> #batch_get_struct<'a> {
                #batch_get_struct::new(self)
            }

            fn delete_item(&'a self) -> #delete_item_hash_key_struct<'a> {
                #delete_item_struct::new(self)
            }

            fn batch_write(&'a self) -> #batch_write_struct<'a> {
                #batch_write_struct::new(self)
            }

            fn condition_check(&'a self) -> #condition_check_hash_key_struct<'a> {
                #condition_check_struct::new(self)
            }
        }
    }
}
