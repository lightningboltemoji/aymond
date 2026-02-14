use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Attribute, Data, DeriveInput, Expr, Fields, GenericArgument, Ident, Lit, Meta, MetaNameValue,
    PathArguments, Token, Type, TypePath, parse_quote, punctuated::Punctuated,
};

pub struct ItemAttribute {
    pub ident: Ident,
    pub attr_name: String,
    pub ty: Type,
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
    pub fn insert_into_map(&self, ident: &TokenStream, map: &TokenStream) -> TokenStream {
        let attr_name = &self.attr_name;
        let boxer = self.into_attribute_value(ident);
        if self.type_paths()[0].as_str() == "Option" {
            parse_quote! {
                if #ident.is_some() {
                    #map.insert(#attr_name.to_string(), #boxer);
                }
            }
        } else {
            parse_quote! {
                #map.insert(#attr_name.to_string(), #boxer);
            }
        }
    }

    pub fn into_attribute_value(&self, ident: &TokenStream) -> Expr {
        let mut typ = self.type_paths();
        Self::into_attribute_value_inner(ident, &mut typ)
    }

    fn into_attribute_value_inner(ident: &TokenStream, typ: &mut Vec<String>) -> Expr {
        match typ.remove(0).as_str() {
            "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128" => {
                parse_quote! {
                    ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue::N(#ident.to_string())
                }
            }
            "String" => parse_quote! {
                ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue::S(#ident.to_string())
            },
            "Vec" => {
                let rec = ItemAttribute::into_attribute_value_inner(&parse_quote!(e), typ);
                parse_quote! {
                    ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue::L(
                        #ident.iter().map(|e| #rec).collect()
                    )
                }
            }
            "Option" => Self::into_attribute_value_inner(&parse_quote!(#ident.unwrap()), typ),
            // We assume this is a struct if it's otherwise not recognized
            _ => parse_quote! {
                ::aymond::shim::aws_sdk_dynamodb::types::AttributeValue::M(#ident.into())
            },
        }
    }

    pub fn from_attribute_value(&self, ident: &Expr) -> Expr {
        let mut typ = self.type_paths();
        Self::from_attribute_value_inner(ident, &mut typ, false)
    }

    pub fn from_attribute_value_inner(
        ident: &Expr,
        typ: &mut Vec<String>,
        as_option: bool,
    ) -> Expr {
        match typ.remove(0).as_str() {
            "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128" => {
                if as_option {
                    parse_quote! {
                        #ident.as_n().ok().map(|e| e.parse().unwrap())
                    }
                } else {
                    parse_quote! {
                        #ident.as_n().unwrap().parse().unwrap()
                    }
                }
            }
            "String" => {
                if as_option {
                    parse_quote! {
                        #ident.as_s().ok().map(|e| e.to_string())
                    }
                } else {
                    parse_quote! {
                        #ident.as_s().unwrap().to_string()
                    }
                }
            }
            "Vec" => {
                let rec = Self::from_attribute_value_inner(&parse_quote!(e), typ, false);
                if as_option {
                    parse_quote! {
                        #ident.as_l().ok().map(|l| l.iter().map(|e| #rec).collect())
                    }
                } else {
                    parse_quote! {
                        #ident.as_l().unwrap().iter().map(|e| #rec).collect()
                    }
                }
            }
            "Option" => Self::from_attribute_value_inner(ident, typ, true),
            // We assume this is a struct if it's otherwise not recognized
            _ => {
                parse_quote! {
                    #ident.as_m().unwrap().into()
                }
            }
        }
    }

    pub fn scalar_type(&self) -> Expr {
        let typ = self.ty.to_token_stream().to_string();
        match typ.as_str() {
            "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128" => {
                parse_quote! {::aymond::shim::aws_sdk_dynamodb::types::ScalarAttributeType::N}
            }
            "String" => {
                parse_quote! {::aymond::shim::aws_sdk_dynamodb::types::ScalarAttributeType::S}
            }
            _ => panic!("Unknown variable type: {}", typ),
        }
    }

    pub fn ty_non_option(&self) -> &Type {
        if let Type::Path(TypePath { path, .. }) = &self.ty {
            let last_segment = path.segments.last();
            if last_segment.is_none() || last_segment.unwrap().ident != "Option" {
                return &self.ty;
            }

            if let PathArguments::AngleBracketed(args) = &last_segment.unwrap().arguments {
                if let Some(GenericArgument::Type(inner_ty)) = args.args.first() {
                    return inner_ty;
                }
            }
        }
        &self.ty
    }

    pub fn type_paths(&self) -> Vec<String> {
        fn collect(ty: &Type, idents: &mut Vec<String>) {
            if let Type::Path(type_path) = ty {
                for segment in &type_path.path.segments {
                    idents.push(segment.ident.to_string());

                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        for arg in &args.args {
                            if let GenericArgument::Type(inner_ty) = arg {
                                collect(inner_ty, idents);
                            }
                        }
                    }
                }
            }
        }
        let mut idents = vec![];
        collect(&self.ty, &mut idents);
        idents
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
            let item_attribute = ItemAttribute {
                ident: field.ident.clone().unwrap(),
                attr_name,
                ty,
            };

            if hash.is_some() {
                hash_key = Some(item_attribute);
            } else if sort.is_some() {
                sort_key = Some(item_attribute);
            } else {
                other_attributes.push(item_attribute);
            }

            if hash_key.is_some()
                && &hash_key.as_ref().unwrap().ty != hash_key.as_ref().unwrap().ty_non_option()
            {
                panic!("Hash key cannot be Option type");
            } else if sort_key.is_some()
                && &sort_key.as_ref().unwrap().ty != sort_key.as_ref().unwrap().ty_non_option()
            {
                panic!("Sort key cannot be Option type");
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
            let field_name = field.ident.as_ref().unwrap().to_string();
            let attribute = field.attrs.iter().find(|a| a.path().is_ident("attribute"));
            let attr_name = attribute
                .map(extract_attributes)
                .and_then(|mut a| a.remove("name"))
                .unwrap_or(field_name);

            let ty = field.ty.clone();
            let item_attribute = ItemAttribute {
                ident: field.ident.clone().unwrap(),
                attr_name,
                ty,
            };

            attributes.push(item_attribute);

            field
                .attrs
                .retain(|attr_def| !attr_def.path().is_ident("attribute"));
        }

        NestedItemDefinition { attributes }
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
