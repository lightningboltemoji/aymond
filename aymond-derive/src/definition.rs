use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident};
use std::collections::HashMap;
use syn::{
    Data, DeriveInput, Expr, Fields, GenericArgument, Ident, Lit, LitStr, Meta, MetaList,
    MetaNameValue, Path, PathArguments, Token, Type, parse_quote, punctuated::Punctuated,
};

#[derive(Clone)]
pub struct ItemAttribute {
    pub field: Ident,
    pub ddb_name: String,
    pub ty: Type,
    pub is_option: bool,
    pub generics_hierarchy: Vec<String>,
}

pub enum GsiRole {
    HashKey,
    SortKey,
}

pub struct GsiDefinition {
    pub name: String,
    pub hash_key: Option<ItemAttribute>,
    pub sort_key: Option<ItemAttribute>,
}

pub struct LsiDefinition {
    pub name: String,
    pub sort_key: ItemAttribute,
}

pub struct ItemDefinition {
    pub name: String,
    pub hash_key: Option<ItemAttribute>,
    pub sort_key: Option<ItemAttribute>,
    pub other_attributes: Vec<ItemAttribute>,
    pub global_secondary_indexes: HashMap<String, GsiDefinition>,
    pub local_secondary_indexes: HashMap<String, LsiDefinition>,
}

impl ItemAttribute {
    pub fn new(field: Ident, ddb_name: String, ty: Type) -> Self {
        let generics_hierarchy = Self::generics_hierarchy(&ty);
        let is_option = generics_hierarchy[0] == "Option";
        ItemAttribute {
            field,
            ddb_name,
            ty,
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
        match &self.generics_hierarchy[hier..] {
            [t, ..]
                if matches!(
                    t.as_str(),
                    "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128"
                ) =>
            {
                parse_quote! (#attr_val::N(#ident.to_string()))
            }
            [t, ..] if t == "String" => parse_quote!(#attr_val::S(#ident.to_string())),
            [h, s, ..] if h == "HashSet" && s == "String" => {
                parse_quote!(#attr_val::Ss(#ident.iter().cloned().collect()))
            }
            [h, v, u, ..] if h == "HashSet" && v == "Vec" && u == "u8" => {
                let blob: TokenStream =
                    parse_quote!(::aymond::shim::aws_sdk_dynamodb::primitives::Blob);
                parse_quote!(#attr_val::Bs(#ident.iter().map(|e| #blob::new(e.clone())).collect()))
            }
            [v, u, ..] if v == "Vec" && u == "u8" => {
                let blob: TokenStream =
                    parse_quote!(::aymond::shim::aws_sdk_dynamodb::primitives::Blob);
                parse_quote!(#attr_val::B(#blob::new(#ident)))
            }
            [v, ..] if v == "Vec" => {
                let rec = self.to_attribute_value_inner(&parse_quote!(e), hier + 1);
                parse_quote!(#attr_val::L(#ident.iter().map(|e| #rec).collect()))
            }
            [t, ..] if t == "Option" => {
                self.to_attribute_value_inner(&parse_quote!(#ident.unwrap()), hier + 1)
            }
            // We assume this is a struct if it's otherwise not recognized
            _ => parse_quote!(#attr_val::M(#ident.into())),
        }
    }

    pub fn unwrap_attribute_value(&self, ident: &Expr) -> Expr {
        self.unwrap_attribute_value_inner(ident, if self.is_option { 1 } else { 0 })
    }

    fn unwrap_attribute_value_inner(&self, ident: &Expr, hier: usize) -> Expr {
        let (as_, get_value): (TokenStream, TokenStream) = match &self.generics_hierarchy[hier..] {
            [t, ..]
                if matches!(
                    t.as_str(),
                    "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128"
                ) =>
            {
                (parse_quote!(.as_n()), parse_quote!(.parse().unwrap()))
            }
            [t, ..] if t == "String" => (parse_quote!(.as_s()), parse_quote!(.to_string())),
            [h, s, ..] if h == "HashSet" && s == "String" => (
                parse_quote!(.as_ss()),
                parse_quote!(.iter().cloned().collect()),
            ),
            [h, v, u, ..] if h == "HashSet" && v == "Vec" && u == "u8" => (
                parse_quote!(.as_bs()),
                parse_quote!(.iter().map(|b| b.clone().into_inner()).collect()),
            ),
            [v, u, ..] if v == "Vec" && u == "u8" => {
                (parse_quote!(.as_b()), parse_quote!(.clone().into_inner()))
            }
            [v, ..] if v == "Vec" => {
                let rec = self.unwrap_attribute_value_inner(&parse_quote!(e), hier + 1);
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
        match self.generics_hierarchy.as_slice() {
            [t] if matches!(
                t.as_str(),
                "i8" | "i16" | "i32" | "i64" | "i128" | "u8" | "u16" | "u32" | "u64" | "u128"
            ) =>
            {
                parse_quote! {::aymond::shim::aws_sdk_dynamodb::types::ScalarAttributeType::N}
            }
            [t] if t == "String" => {
                parse_quote! {::aymond::shim::aws_sdk_dynamodb::types::ScalarAttributeType::S}
            }
            [v, u] if v == "Vec" && u == "u8" => {
                parse_quote! {::aymond::shim::aws_sdk_dynamodb::types::ScalarAttributeType::B}
            }
            _ => panic!("Unknown variable type: {}", self.ty.to_token_stream()),
        }
    }
}

impl ItemAttribute {
    /// Returns the TokenStream for the condition path return type based on generics_hierarchy.
    /// `hier` is the starting index into generics_hierarchy (used to skip Option wrapper).
    pub fn condition_path_type(&self) -> TokenStream {
        Self::condition_path_type_from_hierarchy(&self.generics_hierarchy, 0)
    }

    fn condition_path_type_from_hierarchy(hierarchy: &[String], hier: usize) -> TokenStream {
        let cond: TokenStream = parse_quote!(::aymond::condition);
        match &hierarchy[hier..] {
            [t, ..] if t == "Option" => {
                Self::condition_path_type_from_hierarchy(hierarchy, hier + 1)
            }
            [t, ..]
                if matches!(
                    t.as_str(),
                    "i8" | "i16"
                        | "i32"
                        | "i64"
                        | "i128"
                        | "u8"
                        | "u16"
                        | "u32"
                        | "u64"
                        | "u128"
                        | "bool"
                ) =>
            {
                let ty: TokenStream = t.parse().unwrap();
                parse_quote!(#cond::ScalarConditionPath<#ty>)
            }
            [t, ..] if t == "String" => {
                parse_quote!(#cond::ScalarConditionPath<String>)
            }
            [v, u, ..] if v == "Vec" && u == "u8" => {
                parse_quote!(#cond::ScalarConditionPath<Vec<u8>>)
            }
            [v, ..] if v == "Vec" => {
                let inner = Self::condition_path_type_from_hierarchy(hierarchy, hier + 1);
                parse_quote!(#cond::ListConditionPath<#inner>)
            }
            [h, s, ..] if h == "HashSet" && s == "String" => {
                parse_quote!(#cond::StringSetConditionPath)
            }
            [h, v, u, ..] if h == "HashSet" && v == "Vec" && u == "u8" => {
                parse_quote!(#cond::BinarySetConditionPath)
            }
            // Assume nested struct
            [name, ..] => {
                let path_ident = format_ident!("{}ConditionPath", name);
                parse_quote!(#path_ident)
            }
            [] => panic!("Empty generics_hierarchy"),
        }
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
        let mut gsis: HashMap<String, GsiDefinition> = HashMap::new();
        let mut lsis: HashMap<String, LsiDefinition> = HashMap::new();

        for field in &mut fields_named.named {
            let aymond_attrs: Vec<_> = field
                .attrs
                .iter()
                .filter(|a| a.path().is_ident("aymond"))
                .collect();

            let mut is_hash = false;
            let mut is_sort = false;
            let mut custom_name = None;
            let mut gsi_entries: Vec<(String, GsiRole)> = vec![];
            let mut lsi_entries: Vec<String> = vec![];

            for attr in &aymond_attrs {
                let inner: Meta = attr
                    .parse_args()
                    .expect("Invalid #[aymond(...)] field annotation");
                match &inner {
                    Meta::Path(p) if p.is_ident("hash_key") => is_hash = true,
                    Meta::Path(p) if p.is_ident("sort_key") => is_sort = true,
                    Meta::List(list) if list.path.is_ident("hash_key") => {
                        is_hash = true;
                        custom_name = Self::extract_attribute_name(list);
                    }
                    Meta::List(list) if list.path.is_ident("sort_key") => {
                        is_sort = true;
                        custom_name = Self::extract_attribute_name(list);
                    }
                    Meta::List(list) if list.path.is_ident("attribute") => {
                        custom_name = Self::extract_attribute_name(list);
                    }
                    Meta::List(list)
                        if list.path.is_ident("global_secondary_index")
                            || list.path.is_ident("gsi") =>
                    {
                        gsi_entries.push(Self::parse_gsi_args(list));
                    }
                    Meta::List(list)
                        if list.path.is_ident("local_secondary_index")
                            || list.path.is_ident("lsi") =>
                    {
                        lsi_entries.push(Self::parse_lsi_args(list));
                    }
                    _ => panic!(
                        "Unknown #[aymond(...)] field annotation. Expected hash_key, sort_key, attribute(...), global_secondary_index(...)/gsi(...), or local_secondary_index(...)/lsi(...)"
                    ),
                }
            }

            if is_hash && hash_key.is_some() {
                panic!("Multiple attributes with #[aymond(hash_key)]");
            } else if is_sort && sort_key.is_some() {
                panic!("Multiple attributes with #[aymond(sort_key)]");
            }

            let field_name = field.ident.as_ref().unwrap().to_string();
            let attr_name = custom_name.unwrap_or_else(|| field_name.clone());

            let ty = field.ty.clone();
            let item_attribute = ItemAttribute::new(field.ident.clone().unwrap(), attr_name, ty);

            for (idx_name, role) in gsi_entries {
                let def = gsis.entry(idx_name.clone()).or_insert_with(|| GsiDefinition {
                    name: idx_name.clone(),
                    hash_key: None,
                    sort_key: None,
                });
                match role {
                    GsiRole::HashKey => def.hash_key = Some(item_attribute.clone()),
                    GsiRole::SortKey => def.sort_key = Some(item_attribute.clone()),
                }
            }
            for idx_name in lsi_entries {
                lsis.insert(
                    idx_name.clone(),
                    LsiDefinition {
                        name: idx_name,
                        sort_key: item_attribute.clone(),
                    },
                );
            }

            if is_hash {
                hash_key = Some(item_attribute);
            } else if is_sort {
                sort_key = Some(item_attribute);
            } else {
                other_attributes.push(item_attribute);
            }

            if hash_key.as_ref().filter(|e| e.is_option).is_some() {
                panic!("Hash key cannot be Option type");
            } else if sort_key.as_ref().filter(|e| e.is_option).is_some() {
                panic!("Sort key cannot be Option type");
            }

            field
                .attrs
                .retain(|attr_def| !attr_def.path().is_ident("aymond"));
        }

        if !nested {
            hash_key
                .as_ref()
                .expect("#[aymond(hash_key)] must be defined");
        }

        ItemDefinition {
            name,
            hash_key,
            sort_key,
            other_attributes,
            global_secondary_indexes: gsis,
            local_secondary_indexes: lsis,
        }
    }

    pub fn all_attributes(&self) -> impl Iterator<Item = &ItemAttribute> {
        self.hash_key
            .iter()
            .chain(self.sort_key.iter())
            .chain(self.other_attributes.iter())
    }

    fn parse_gsi_args(list: &MetaList) -> (String, GsiRole) {
        struct GsiArgs {
            name: LitStr,
            role: Path,
        }
        impl syn::parse::Parse for GsiArgs {
            fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
                let name: LitStr = input.parse()?;
                let _: Token![,] = input.parse()?;
                let role: Path = input.parse()?;
                Ok(GsiArgs { name, role })
            }
        }
        let args: GsiArgs = list
            .parse_args()
            .expect("Invalid GSI annotation. Expected: gsi(\"name\", hash_key|sort_key)");
        let name = args.name.value();
        let role = if args.role.is_ident("hash_key") {
            GsiRole::HashKey
        } else if args.role.is_ident("sort_key") {
            GsiRole::SortKey
        } else {
            panic!("Invalid GSI role. Expected hash_key or sort_key")
        };
        (name, role)
    }

    fn parse_lsi_args(list: &MetaList) -> String {
        let name: LitStr = list
            .parse_args()
            .expect("Invalid LSI annotation. Expected: lsi(\"name\")");
        name.value()
    }

    fn extract_attribute_name(list: &MetaList) -> Option<String> {
        list.parse_args_with(Punctuated::<MetaNameValue, Token![,]>::parse_terminated)
            .ok()?
            .into_iter()
            .find_map(|nv| {
                if nv.path.is_ident("name")
                    && let Expr::Lit(expr_lit) = &nv.value
                    && let Lit::Str(s) = &expr_lit.lit
                {
                    return Some(s.value());
                }
                None
            })
    }
}
