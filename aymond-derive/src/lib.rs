use proc_macro::TokenStream;
use quote::quote;
use std::collections::HashMap;
use syn::{
    parse::Parser, parse_macro_input, parse_quote, punctuated::Punctuated, Attribute, Data,
    DeriveInput, Expr, Field, Fields, GenericArgument, Ident, Lit, Meta, MetaNameValue,
    PathArguments, Token, Type,
};

struct ItemAttribute {
    ident: Ident,
    attr_name: String,
    ty: Type,
    typ: String,
    typ_ident: Ident,
}

struct ItemDefinition {
    hash_key: ItemAttribute,
    sort_key: Option<ItemAttribute>,
    other_attributes: Vec<ItemAttribute>,
}

struct NestedItemDefinition {
    attributes: Vec<ItemAttribute>,
}

impl ItemAttribute {
    fn box_unbox_inner(ident: &Ident, typ: &mut Vec<String>) -> (Expr, Expr) {
        match typ.remove(0).as_str() {
            "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128" => (
                parse_quote! {
                    ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue::N(#ident.to_string())
                },
                parse_quote! {
                    #ident.parse().unwrap()
                },
            ),
            "String" => (
                parse_quote! {
                    ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue::S(#ident.to_string())
                },
                parse_quote! {
                    #ident.as_s().unwrap().to_string()
                },
            ),
            "Vec" => {
                let (rec_box, rec_unbox) = ItemAttribute::box_unbox_inner(ident, typ);
                (
                    parse_quote! {
                        ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue::L(
                            #ident.iter().map(|#ident| #rec_box).collect()
                        )
                    },
                    parse_quote! {
                        #ident.as_l().unwrap().iter().map(|#ident| #rec_unbox).collect()
                    },
                )
            }
            // We assume this is a struct if it's otherwise not recognized
            _ => (
                parse_quote! {
                    ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue::M(#ident.into())
                },
                parse_quote! {
                    #ident.into()
                },
            ),
        }
    }

    fn box_unbox(&self) -> (Expr, Expr) {
        let attr_name = &self.attr_name;
        let field_ident = &self.ident;
        let mut typ: Vec<String> = vec![];
        collect_type_idents(&self.ty, &mut typ);
        match typ.remove(0).as_str() {
            "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128" => (
                parse_quote! {
                    ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue::N(self.#field_ident.to_string())
                },
                parse_quote! {
                    map.get(#attr_name).unwrap().as_n().unwrap().parse().unwrap()
                },
            ),
            "String" => (
                parse_quote! {
                    ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue::S(self.#field_ident.to_string())
                },
                parse_quote! {
                    map.get(#attr_name).unwrap().as_s().unwrap().to_string()
                },
            ),
            "Vec" => {
                let e = parse_quote!(e);
                let (rec_box, rec_unbox) = ItemAttribute::box_unbox_inner(&e, &mut typ);
                (
                    parse_quote! {
                        ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue::L(
                            self.#field_ident.iter().map(|#e| #rec_box).collect()
                        )
                    },
                    parse_quote! {
                        map.get(#attr_name).unwrap().as_l().unwrap().iter().map(|#e| #rec_unbox).collect()
                    },
                )
            }
            // We assume this is a struct if it's otherwise not recognized
            _ => (
                parse_quote! {
                    ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue::M(self.#field_ident.into())
                },
                parse_quote! {
                    map.get(#attr_name).unwrap().as_m().unwrap().into()
                },
            ),
        }
    }

    fn key_boxer(&self) -> Expr {
        let field_ident = &self.ident;
        match self.typ.as_str() {
            "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128" =>
                parse_quote! {
                    ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue::N(#field_ident.into().to_string())
                },
            "String" => parse_quote! {
                ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue::S(#field_ident.into())
            },
            _ => panic!(
                "Type cannot be used for a DynamoDB key (S, N, B only): {}",
                self.typ.as_str()
            ),
        }
    }

    fn scalar_type(&self) -> Expr {
        match self.typ.as_str() {
            "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128" => {
                parse_quote! {::aymond::shim::aws_sdk_dynamodb::types::ScalarAttributeType::N}
            }
            "String" => {
                parse_quote! {::aymond::shim::aws_sdk_dynamodb::types::ScalarAttributeType::S}
            }
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

            let hash = field.attrs.iter().find(|a| a.path().is_ident("hash_key"));
            let sort = field.attrs.iter().find(|a| a.path().is_ident("sort_key"));
            let attribute = field.attrs.iter().find(|a| a.path().is_ident("attribute"));

            if hash.is_some() && hash_key.is_some() {
                panic!("Multiple attributes with #[hash_key]");
            } else if sort.is_some() && sort_key.is_some() {
                panic!("Multiple attributes with #[sort_key]");
            }

            let field_name = field.ident.as_ref().unwrap().to_string();
            let source = hash.or(sort).or(attribute);
            let attr_name = source
                .map(extract_attributes)
                .and_then(|mut a| a.remove("name"))
                .unwrap_or(field_name);

            let ty = field.ty.clone();
            let typ_ident = path.path.segments.first().unwrap().ident.clone();
            let typ = path.path.segments.first().unwrap().ident.to_string();
            let item_attribute = ItemAttribute {
                ident: field.ident.clone().unwrap(),
                attr_name,
                ty,
                typ,
                typ_ident,
            };

            if hash.is_some() {
                hash_key = Some(item_attribute);
            } else if sort.is_some() {
                sort_key = Some(item_attribute);
            } else {
                other_attributes.push(item_attribute);
            }

            field.attrs.retain(|attr_def| {
                !attr_def.path().is_ident("hash_key")
                    && !attr_def.path().is_ident("sort_key")
                    && !attr_def.path().is_ident("attribute")
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

fn collect_type_idents(ty: &Type, idents: &mut Vec<String>) {
    if let Type::Path(type_path) = ty {
        for segment in &type_path.path.segments {
            idents.push(segment.ident.to_string());

            if let PathArguments::AngleBracketed(args) = &segment.arguments {
                for arg in &args.args {
                    if let GenericArgument::Type(inner_ty) = arg {
                        collect_type_idents(inner_ty, idents);
                    }
                }
            }
        }
    }
}

impl From<&mut DeriveInput> for NestedItemDefinition {
    fn from(ast: &mut DeriveInput) -> Self {
        let data_struct = match &mut ast.data {
            Data::Struct(data_struct) => data_struct,
            _ => panic!("Only structs are supported"),
        };
        let fields_named = match &mut data_struct.fields {
            Fields::Named(fields_named) => fields_named,
            _ => panic!("Only named fields are supported"),
        };

        let mut attributes = vec![];

        for field in &mut fields_named.named {
            let path = match &field.ty {
                Type::Path(path) => path,
                _ => panic!("Unknown path type"),
            };

            let field_name = field.ident.as_ref().unwrap().to_string();
            let attribute = field.attrs.iter().find(|a| a.path().is_ident("attribute"));
            let attr_name = attribute
                .map(extract_attributes)
                .and_then(|mut a| a.remove("name"))
                .unwrap_or(field_name);

            let ty = field.ty.clone();
            let typ_ident = path.path.segments.first().unwrap().ident.clone();
            let typ = path.path.segments.first().unwrap().ident.to_string();
            let item_attribute = ItemAttribute {
                ident: field.ident.clone().unwrap(),
                attr_name,
                ty,
                typ,
                typ_ident,
            };

            attributes.push(item_attribute);

            field
                .attrs
                .retain(|attr_def| !attr_def.path().is_ident("attribute"));
        }

        NestedItemDefinition { attributes }
    }
}

#[proc_macro_attribute]
pub fn item(_args: TokenStream, input: TokenStream) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

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
    let mut attr_typ_ident: Vec<Ident> = vec![];
    let mut key_boxer: Vec<Expr> = vec![];

    let mut append = |i: ItemAttribute, key: bool| {
        let (boxer, unboxer) = i.box_unbox();
        attr_boxer.push(boxer);
        attr_unboxer.push(unboxer);
        if key {
            key_boxer.push(i.key_boxer());
        }
        attr_ident.push(i.ident);
        attr_name.push(i.attr_name);
        attr_typ_ident.push(i.typ_ident);
    };

    let has_sort_key = def.sort_key.is_some();
    append(def.hash_key, true);
    def.sort_key.into_iter().for_each(|e| append(e, true));
    def.other_attributes
        .into_iter()
        .for_each(|e| append(e, false));

    let key_ident = &attr_ident[0..(if has_sort_key { 2 } else { 1 })];
    let key_attr_name = &attr_name[0..(if has_sort_key { 2 } else { 1 })];
    let key_attr_ident = &attr_typ_ident[0..(if has_sort_key { 2 } else { 1 })];
    let key_type: Vec<Expr> = (|| {
        let mut v = vec![parse_quote! {#aws_sdk_dynamodb::types::KeyType::Hash}];
        if has_sort_key {
            v.push(parse_quote! {#aws_sdk_dynamodb::types::KeyType::Range});
        }
        v
    })();

    quote! {
        #[derive(Debug)]
        #input

        impl From<&::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>> for #name {
            fn from(map: &::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>) -> Self {
                #name {
                    #( #attr_ident: #attr_unboxer ),*
                }
            }
        }

        impl Into<::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>> for #name {
            fn into(self) -> ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue> {
                let mut map = ::std::collections::HashMap::new();
                #(
                    map.insert(#attr_name.to_string(), #attr_boxer);
                )*
                map
            }
        }

        impl #name {
            pub fn key(
                #(
                    #key_ident: impl Into<#key_attr_ident>
                ),*
            ) -> ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue> {
                let mut map = ::std::collections::HashMap::new();
                #(
                    map.insert(#key_attr_name.to_string(), #key_boxer);
                )*
                map
            }
        }

        impl Item for #name {
            fn key_schemas() -> Vec<#aws_sdk_dynamodb::types::KeySchemaElement> {
                vec![
                    #(
                        #aws_sdk_dynamodb::types::KeySchemaElement::builder()
                            .attribute_name(#key_attr_name)
                            .key_type(#key_type)
                            .build()
                            .unwrap()
                    ),*
                ]
            }

            fn key_attribute_defintions() -> Vec<#aws_sdk_dynamodb::types::AttributeDefinition> {
                vec![
                    #(
                        #aws_sdk_dynamodb::types::AttributeDefinition::builder()
                            .attribute_name(#key_attr_name)
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
pub fn nested_item(_args: TokenStream, input: TokenStream) -> TokenStream {
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let mut input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident.clone();
    let def: NestedItemDefinition = (&mut input).into();

    let mut attr_ident: Vec<Ident> = vec![];
    let mut attr_name: Vec<String> = vec![];
    let mut attr_boxer: Vec<Expr> = vec![];
    let mut attr_unboxer: Vec<Expr> = vec![];
    let mut attr_typ_ident: Vec<Ident> = vec![];

    let mut append = |i: ItemAttribute| {
        let (boxer, unboxer) = i.box_unbox();
        attr_boxer.push(boxer);
        attr_unboxer.push(unboxer);
        attr_ident.push(i.ident);
        attr_name.push(i.attr_name);
        attr_typ_ident.push(i.typ_ident);
    };

    def.attributes.into_iter().for_each(|e| append(e));

    quote! {
        #[derive(Debug)]
        #input

        impl From<&::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>> for #name {
            fn from(map: &::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>) -> Self {
                #name {
                    #( #attr_ident: #attr_unboxer ),*
                }
            }
        }

        impl Into<::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>> for #name {
            fn into(self) -> ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue> {
                let mut map = ::std::collections::HashMap::new();
                #(
                    map.insert(#attr_name.to_string(), #attr_boxer);
                )*
                map
            }
        }
    }.into()
}
#[proc_macro_attribute]
pub fn table(args: TokenStream, input: TokenStream) -> TokenStream {
    let aws_types: Expr = parse_quote!(::aymond::shim::aws_types);
    let aws_sdk_dynamodb: Expr = parse_quote!(::aymond::shim::aws_sdk_dynamodb);

    let mut input = parse_macro_input!(input as DeriveInput);
    let typ = parse_macro_input!(args as Ident);

    if let Data::Struct(ref mut data_struct) = input.data {
        if let Fields::Named(ref mut fields) = data_struct.fields {
            fields.named.push(
                Field::parse_named
                    .parse2(quote! { client: ::std::sync::Arc<#aws_sdk_dynamodb::Client> })
                    .unwrap(),
            );
            fields.named.push(
                Field::parse_named
                    .parse2(quote! { table_name: String }).unwrap(),
            );
        }
    }

    let name = &input.ident;


    quote! {
        #[derive(Debug)]
        #input

        impl Table<#typ> for #name {

            fn new_with_local_config(
                table_name: impl Into<String>,
                endpoint_url: impl Into<String>,
                region_name: impl Into<String>,
            ) -> Self {
                let credentials = ::aymond::shim::aws_credential_types::Credentials::from_keys("empty", "empty", None);
                let table_name = table_name.into();
                let endpoint_url = endpoint_url.into();
                let region_name = region_name.into();
                Self::new_with_config_builder(table_name, move |b| {
                    b.credentials_provider(#aws_types::sdk_config::SharedCredentialsProvider::new(credentials))
                        .region(#aws_types::region::Region::new(region_name))
                        .endpoint_url(endpoint_url)
                        .behavior_version(#aws_sdk_dynamodb::config::BehaviorVersion::latest())
                })
            }

            fn new_with_config_builder<F>(table_name: impl ::core::convert::Into<String>, builder: F) -> Self
            where
                F: FnOnce(#aws_types::sdk_config::Builder) -> #aws_types::sdk_config::Builder {
                    let config = builder(#aws_types::SdkConfig::builder()).build();
                    Self::new_with_config(table_name, config)
                }

            async fn new_with_default_config(table_name: impl ::core::convert::Into<String>) -> Self {
                let config = ::aymond::shim::aws_config::load_defaults(
                    ::aymond::shim::aws_config::BehaviorVersion::latest()
                ).await;
                Self::new_with_config(table_name, config)
            }

            fn new_with_config(table_name: impl ::core::convert::Into<String>, config: #aws_types::SdkConfig) -> Self {
                let client = ::std::sync::Arc::new(#aws_sdk_dynamodb::Client::new(&config));
                Self::new_with_client(table_name, client)
            }

            fn new_with_client(
                table_name: impl ::core::convert::Into<String>,
                client: ::std::sync::Arc<#aws_sdk_dynamodb::Client>,
            ) -> Self {
                Self {
                    client,
                    table_name: table_name.into()
                }
            }

            async fn create(&self, err_if_exists: bool) -> Result<
                (), #aws_sdk_dynamodb::error::SdkError<
                    #aws_sdk_dynamodb::operation::create_table::CreateTableError,
                    #aws_sdk_dynamodb::config::http::HttpResponse
                >
            > {
                let res = self.client.create_table()
                    .table_name(&self.table_name)
                    .set_key_schema(Some(#typ::key_schemas()))
                    .set_attribute_definitions(Some(#typ::key_attribute_defintions()))
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

            async fn get(
                &self,
                key: ::std::collections::HashMap<String, #aws_sdk_dynamodb::types::AttributeValue>
            ) -> Result<
                #aws_sdk_dynamodb::operation::get_item::GetItemOutput,
                #aws_sdk_dynamodb::error::SdkError<
                    #aws_sdk_dynamodb::operation::get_item::GetItemError,
                    #aws_sdk_dynamodb::config::http::HttpResponse
                >
            > {
                self.client.get_item()
                    .table_name(&self.table_name)
                    .set_key(Some(key))
                    .send()
                    .await
            }

            async fn put(&self, t: #typ) -> Result<
                #aws_sdk_dynamodb::operation::put_item::PutItemOutput,
                #aws_sdk_dynamodb::error::SdkError<
                    #aws_sdk_dynamodb::operation::put_item::PutItemError,
                    #aws_sdk_dynamodb::config::http::HttpResponse
                >
            > {
                self.client.put_item()
                    .table_name(&self.table_name)
                    .set_item(Some(t.into()))
                    .send()
                    .await
            }
        }
    }
    .into()
}
