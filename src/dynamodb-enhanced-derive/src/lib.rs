use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashMap;
use syn::{
    Attribute, Data, DeriveInput, Expr, Field, Fields, Ident, Lit, Meta, MetaNameValue, Token,
    Type, parse::Parser, parse_macro_input, parse_quote, punctuated::Punctuated,
};

struct ItemAttribute {
    ident: Ident,
    attr_name: String,
    typ: String,
}

struct ItemDefinition {
    hash_key: ItemAttribute,
    sort_key: Option<ItemAttribute>,
    other_attributes: Vec<ItemAttribute>,
}

impl ItemAttribute {
    fn box_unbox(&self) -> (Expr, Expr) {
        let field_name = self.ident.to_string();
        let field_ident = &self.ident;
        match self.typ.as_str() {
            "i32" | "i64" | "i128" => (
                parse_quote! {
                    ::aws_sdk_dynamodb::types::AttributeValue::N(self.#field_ident.to_string())
                },
                parse_quote! {
                    map.get(#field_name).unwrap().as_n().unwrap().parse().unwrap()
                },
            ),
            "String" => (
                parse_quote! {
                    ::aws_sdk_dynamodb::types::AttributeValue::S(self.#field_ident.to_string())
                },
                parse_quote! {
                    map.get(#field_name).unwrap().as_s().unwrap().to_string()
                },
            ),
            _ => panic!("Unknown variable type: {}", self.typ.as_str()),
        }
    }

    fn scalar_type(&self) -> Expr {
        match self.typ.as_str() {
            "i32" | "i64" | "i128" => {
                parse_quote! {::aws_sdk_dynamodb::types::ScalarAttributeType::N}
            }
            "String" => parse_quote! {::aws_sdk_dynamodb::types::ScalarAttributeType::S},
            _ => panic!("Unknown variable type: {}", self.typ.as_str()),
        }
    }
}

impl From<&mut DeriveInput> for ItemDefinition {
    fn from(ast: &mut DeriveInput) -> Self {
        let data_struct = match &mut ast.data {
            Data::Struct(data_struct) => data_struct,
            _ => panic!("Only structs are supported"),
        };
        let fields_named = match &mut data_struct.fields {
            Fields::Named(fields_named) => fields_named,
            _ => panic!("Only named fields are supported"),
        };

        let mut hash_key = None;
        let mut sort_key = None;
        let mut other_attributes = vec![];

        for field in &mut fields_named.named {
            let path = match &field.ty {
                Type::Path(path) => path,
                _ => panic!("Unknown path type"),
            };
            field.attrs.retain(|attr_def| {
                let field_name = field.ident.as_ref().unwrap().to_string();
                let mut attrs = extract_attributes(attr_def);
                let attr_name = attrs.remove("name").unwrap_or_else(|| field_name);
                let typ = path.path.segments.first().unwrap().ident.to_string();
                let create_item_attribute = || ItemAttribute {
                    ident: field.ident.clone().unwrap(),
                    attr_name,
                    typ,
                };

                if attr_def.path().is_ident("hash_key") {
                    if hash_key.is_some() {
                        panic!("Multiple attributes with #[hash_key]");
                    }
                    hash_key = Some(create_item_attribute());
                    false
                } else if attr_def.path().is_ident("sort_key") {
                    if sort_key.is_some() {
                        panic!("Multiple attributes with #[sort_key]");
                    }
                    sort_key = Some(create_item_attribute());
                    false
                } else if attr_def.path().is_ident("attribute") {
                    other_attributes.push(create_item_attribute());
                    false
                } else {
                    true
                }
            });
        }

        ItemDefinition {
            hash_key: hash_key.expect("#[hash_key] must be defined"),
            sort_key,
            other_attributes,
        }
    }
}

fn extract_attributes(attr: &Attribute) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Meta::List(meta_list) = attr.meta.clone() {
        meta_list
            .parse_args_with(Punctuated::parse_terminated)
            .into_iter()
            .for_each(|nested: Punctuated<MetaNameValue, Token![,]>| {
                for nv in nested {
                    let param_name = nv.path.get_ident().unwrap().to_string();
                    let param_value = match &nv.value {
                        Expr::Lit(l) => match &l.lit {
                            Lit::Str(s) => s.value(),
                            _ => panic!("Expected value to be String"),
                        },
                        _ => panic!("Expected value to be literal"),
                    };
                    map.insert(param_name, param_value);
                }
            });
    }
    map
}

#[proc_macro_attribute]
pub fn item(_args: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    // println!("{:#?}", input);
    let name = &input.ident.clone();
    let def: ItemDefinition = (&mut input).into();

    let mut key_scalar_type: Vec<Expr> = vec![];
    key_scalar_type.push(def.hash_key.scalar_type());
    def.sort_key
        .iter()
        .map(|e| e.scalar_type())
        .for_each(|e| key_scalar_type.push(e));

    let mut attr_ident: Vec<Ident> = vec![];
    let mut attr_name: Vec<String> = vec![];
    let mut attr_boxer: Vec<Expr> = vec![];
    let mut attr_unboxer: Vec<Expr> = vec![];

    let mut append = |i: ItemAttribute| {
        let (boxer, unboxer) = i.box_unbox();
        attr_ident.push(i.ident);
        attr_name.push(i.attr_name);
        attr_boxer.push(boxer);
        attr_unboxer.push(unboxer);
    };

    let has_sort_key = def.sort_key.is_some();
    append(def.hash_key);
    def.sort_key.into_iter().for_each(|e| append(e));
    def.other_attributes.into_iter().for_each(|e| append(e));

    // let key_ident = &attr_ident[0..(if has_sort_key { 2 } else { 1 })];
    let key_name = &attr_name[0..(if has_sort_key { 2 } else { 1 })];
    let key_type: Vec<Expr> = (|| {
        let mut v = vec![parse_quote! {::aws_sdk_dynamodb::types::KeyType::Hash}];
        if has_sort_key {
            v.push(parse_quote! {::aws_sdk_dynamodb::types::KeyType::Range});
        }
        v
    })();
    // let key_boxer = &attr_boxer[0..(if has_sort_key { 2 } else { 1 })];
    // let key_unboxer = &attr_unboxer[0..(if has_sort_key { 2 } else { 1 })];

    quote! {
        #input

        impl From<&::std::collections::HashMap<String, ::aws_sdk_dynamodb::types::AttributeValue>> for #name {
            fn from(map: &::std::collections::HashMap<String, ::aws_sdk_dynamodb::types::AttributeValue>) -> Self {
                #name {
                    #( #attr_ident: #attr_unboxer ),*
                }
            }
        }

        impl Into<::std::collections::HashMap<String, ::aws_sdk_dynamodb::types::AttributeValue>> for #name {
            fn into(self) -> ::std::collections::HashMap<String, ::aws_sdk_dynamodb::types::AttributeValue> {
                let mut map = ::std::collections::HashMap::new();
                #(
                    map.insert(#attr_name.to_string(), #attr_boxer);
                )*
                map
            }
        }

        impl Item for #name {
            fn key_schemas() -> Vec<::aws_sdk_dynamodb::types::KeySchemaElement> {
                vec![
                    #(
                        ::aws_sdk_dynamodb::types::KeySchemaElement::builder()
                            .attribute_name(#key_name)
                            .key_type(#key_type)
                            .build()
                            .unwrap()
                    ),*
                ]
            }

            fn key_attribute_defintions() -> Vec<::aws_sdk_dynamodb::types::AttributeDefinition> {
                vec![
                    #(
                        ::aws_sdk_dynamodb::types::AttributeDefinition::builder()
                            .attribute_name(#key_name)
                            .attribute_type(#key_scalar_type)
                            .build()
                            .unwrap()
                    ),*
                ]
            }
        }
    }.into()
}

#[proc_macro_attribute]
pub fn table(args: TokenStream, input: TokenStream) -> TokenStream {
    let mut input = parse_macro_input!(input as DeriveInput);
    let typ = parse_macro_input!(args as Ident);

    if let Data::Struct(ref mut data_struct) = input.data {
        if let Fields::Named(ref mut fields) = data_struct.fields {
            fields.named.push(
                Field::parse_named
                    .parse2(quote! { client: ::std::sync::Arc<::aws_sdk_dynamodb::Client> })
                    .unwrap(),
            );
            fields.named.push(
                Field::parse_named
                    .parse2(quote! { table_name: String })
                    .unwrap(),
            );
        }
    }

    let name = &input.ident;
    quote! {
        #input

        impl Table for #name {
            fn new(client: ::std::sync::Arc<::aws_sdk_dynamodb::Client>, table_name: impl ::core::convert::Into<String>) -> Self {
                #name {
                    client,
                    table_name: table_name.into()
                }
            }

            async fn create_table(&self) -> Result<
                ::aws_sdk_dynamodb::operation::create_table::CreateTableOutput,
                ::aws_sdk_dynamodb::error::SdkError<
                    ::aws_sdk_dynamodb::operation::create_table::CreateTableError,
                    ::aws_sdk_dynamodb::config::http::HttpResponse
                >
            > {
                self.client.create_table()
                    .table_name(&self.table_name)
                    .set_key_schema(Some(#typ::key_schemas()))
                    .set_attribute_definitions(Some(#typ::key_attribute_defintions()))
                    .billing_mode(::aws_sdk_dynamodb::types::BillingMode::PayPerRequest)
                    .send()
                    .await
            }
        }
    }.into()
}
