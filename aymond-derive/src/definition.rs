use std::collections::HashMap;

use syn::{
    Attribute, Data, DeriveInput, Expr, Fields, GenericArgument, Ident, Lit, Meta, MetaNameValue,
    PathArguments, Token, Type, parse_quote, punctuated::Punctuated,
};

pub struct ItemAttribute {
    pub ident: Ident,
    pub attr_name: String,
    pub ty: Type,
    pub typ: String,
    pub typ_ident: Ident,
}

pub struct ItemDefinition {
    pub name: String,
    pub hash_key: ItemAttribute,
    pub sort_key: Option<ItemAttribute>,
    pub other_attributes: Vec<ItemAttribute>,
}

pub struct NestedItemDefinition {
    pub attributes: Vec<ItemAttribute>,
}

impl ItemAttribute {
    pub fn box_unbox_inner(ident: &Ident, typ: &mut Vec<String>) -> (Expr, Expr) {
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

    pub fn box_unbox(&self) -> (Expr, Expr) {
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

    pub fn key_boxer_for(&self, ident: &Expr) -> Expr {
        match self.typ.as_str() {
            "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128" => {
                parse_quote! {
                    ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue::N(#ident.to_string())
                }
            }
            "String" => parse_quote! {
                ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue::S(#ident.into())
            },
            _ => panic!(
                "Type cannot be used for a DynamoDB key (S, N, B only): {}",
                self.typ.as_str()
            ),
        }
    }

    pub fn scalar_type(&self) -> Expr {
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
        let name = ast.ident.to_string();
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
            name,
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
