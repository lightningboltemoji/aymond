use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::{
    Attribute, Data, DeriveInput, Expr, Fields, GenericArgument, Ident, Lit, Meta, MetaNameValue,
    PathArguments, Token, Type, TypePath, parse_quote, punctuated::Punctuated,
};

pub struct ItemAttribute {
    pub field: Ident,
    pub ddb_name: String,
    pub ty: Type,
    pub ty_value: Type,
    pub is_option: bool,
    pub generics_hierarchy: Vec<String>,
}

pub struct ItemDefinition {
    pub name: String,
    pub hash_key: Option<ItemAttribute>,
    pub sort_key: Option<ItemAttribute>,
    pub other_attributes: Vec<ItemAttribute>,
}

impl ItemAttribute {
    pub fn new(field: Ident, ddb_name: String, ty: Type) -> Self {
        let generics_hierarchy = Self::generics_hierarchy(&ty);
        let is_option = generics_hierarchy[0] == "Option";
        let ty_value = Self::ty_value(&ty, is_option);
        ItemAttribute {
            field,
            ddb_name,
            ty,
            ty_value,
            is_option,
            generics_hierarchy,
        }
    }

    pub fn generics_hierarchy(ty: &Type) -> Vec<String> {
        fn collect(ty: &Type, idents: &mut Vec<String>) {
            if let Type::Path(type_path) = ty {
                for segment in &type_path.path.segments {
                    idents.push(segment.ident.to_string());

                    if let PathArguments::AngleBracketed(args) = &segment.arguments {
                        let arg = &args.args.first().unwrap();
                        if let GenericArgument::Type(inner_ty) = arg {
                            collect(inner_ty, idents);
                        }
                    }
                }
            }
        }
        let mut idents = vec![];
        collect(ty, &mut idents);
        idents
    }

    pub fn insert_into_map(&self, ident: &TokenStream, map: &TokenStream) -> TokenStream {
        let attr_name = &self.ddb_name;
        let boxer = self.to_attribute_value(ident);
        let insert: TokenStream = parse_quote!(#map.insert(#attr_name.to_string(), #boxer););
        if self.is_option {
            return parse_quote! {
                if #ident.is_some() {
                    #insert
                }
            };
        }
        insert
    }

    pub fn to_attribute_value(&self, ident: &TokenStream) -> Expr {
        self.to_attribute_value_inner(ident, 0)
    }

    fn to_attribute_value_inner(&self, ident: &TokenStream, hier: usize) -> Expr {
        let attr_val: TokenStream =
            parse_quote!(::aymond::shim::aws_sdk_dynamodb::types::AttributeValue);
        match self.generics_hierarchy[hier].as_str() {
            "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128" => {
                parse_quote! (#attr_val::N(#ident.to_string()))
            }
            "String" => parse_quote!(#attr_val::S(#ident.to_string())),
            "Vec" => {
                let rec = self.to_attribute_value_inner(&parse_quote!(e), hier + 1);
                parse_quote!(#attr_val::L(#ident.iter().map(|e| #rec).collect()))
            }
            "Option" => self.to_attribute_value_inner(&parse_quote!(#ident.unwrap()), hier + 1),
            // We assume this is a struct if it's otherwise not recognized
            _ => parse_quote!(#attr_val::M(#ident.into())),
        }
    }

    pub fn from_attribute_value(&self, ident: &Expr) -> Expr {
        self.from_attribute_value_inner(ident, if self.is_option { 1 } else { 0 })
    }

    fn from_attribute_value_inner(&self, ident: &Expr, hier: usize) -> Expr {
        let (as_, get_value): (TokenStream, TokenStream) =
            match self.generics_hierarchy[hier].as_str() {
                "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128" => {
                    (parse_quote!(.as_n()), parse_quote!(.parse().unwrap()))
                }
                "String" => (parse_quote!(.as_s()), parse_quote!(.to_string())),
                "Vec" => {
                    let rec = self.from_attribute_value_inner(&parse_quote!(e), hier + 1);
                    (
                        parse_quote!(.as_l()),
                        parse_quote!(.iter().map(|e| #rec).collect()),
                    )
                }
                // We assume this is a struct if it's otherwise not recognized
                _ => (parse_quote!(.as_m()), parse_quote!(.into())),
            };

        if hier == 1 && self.is_option {
            parse_quote!(#ident #as_ .ok().map(|e| e #get_value))
        } else {
            parse_quote!(#ident #as_ .unwrap() #get_value)
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

    pub fn ty_value(ty: &Type, is_option: bool) -> Type {
        if is_option
            && let Type::Path(TypePath { path, .. }) = ty
            && let PathArguments::AngleBracketed(args) = &path.segments.first().unwrap().arguments
            && let Some(GenericArgument::Type(inner_ty)) = args.args.first()
        {
            return inner_ty.clone();
        }
        ty.clone()
    }
}

impl ItemDefinition {
    pub fn new(ast: &mut DeriveInput, nested: bool) -> Self {
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
                .map(Self::extract_attributes)
                .and_then(|mut a| a.remove("name"))
                .unwrap_or(field_name);

            let ty = field.ty.clone();
            let item_attribute = ItemAttribute::new(field.ident.clone().unwrap(), attr_name, ty);

            if hash.is_some() {
                hash_key = Some(item_attribute);
            } else if sort.is_some() {
                sort_key = Some(item_attribute);
            } else {
                other_attributes.push(item_attribute);
            }

            if hash_key.as_ref().filter(|e| e.is_option).is_some() {
                panic!("Hash key cannot be Option type");
            } else if sort_key.as_ref().filter(|e| e.is_option).is_some() {
                panic!("Sort key cannot be Option type");
            }

            field.attrs.retain(|attr_def| {
                !attr_def.path().is_ident("hash_key")
                    && !attr_def.path().is_ident("sort_key")
                    && !attr_def.path().is_ident("attribute")
            });
        }

        if !nested {
            hash_key.as_ref().expect("#[hash_key] must be defined");
        }

        ItemDefinition {
            name,
            hash_key,
            sort_key,
            other_attributes,
        }
    }

    pub fn all_attributes(&self) -> impl Iterator<Item = &ItemAttribute> {
        self.hash_key
            .iter()
            .chain(self.sort_key.iter())
            .chain(self.other_attributes.iter())
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
}
