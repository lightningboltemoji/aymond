use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident};
use std::collections::HashMap;
use syn::{
    Data, DeriveInput, Expr, Fields, GenericArgument, Ident, Lit, LitStr, Meta, MetaList, Path,
    PathArguments, Token, Type, parse_quote, punctuated::Punctuated,
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
    pub version_attribute: Option<ItemAttribute>,
}

fn combine_error(errors: &mut Option<syn::Error>, err: syn::Error) {
    if let Some(existing) = errors {
        existing.combine(err);
    } else {
        *errors = Some(err);
    }
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

    pub fn update_path_type(&self) -> TokenStream {
        Self::update_path_type_from_hierarchy(&self.generics_hierarchy, 0)
    }

    fn update_path_type_from_hierarchy(hierarchy: &[String], hier: usize) -> TokenStream {
        let upd: TokenStream = parse_quote!(::aymond::update);
        match &hierarchy[hier..] {
            [t, ..] if t == "Option" => Self::update_path_type_from_hierarchy(hierarchy, hier + 1),
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
                parse_quote!(#upd::ScalarUpdatePath<#ty>)
            }
            [t, ..] if t == "String" => parse_quote!(#upd::ScalarUpdatePath<String>),
            [v, u, ..] if v == "Vec" && u == "u8" => {
                parse_quote!(#upd::ScalarUpdatePath<Vec<u8>>)
            }
            [v, ..] if v == "Vec" => {
                let inner = Self::update_path_type_from_hierarchy(hierarchy, hier + 1);
                parse_quote!(#upd::ListUpdatePath<#inner>)
            }
            [h, s, ..] if h == "HashSet" && s == "String" => {
                parse_quote!(#upd::SetUpdatePath<String>)
            }
            [h, v, u, ..] if h == "HashSet" && v == "Vec" && u == "u8" => {
                parse_quote!(#upd::SetUpdatePath<Vec<u8>>)
            }
            [name, ..] => {
                let path_ident = format_ident!("{}UpdatePath", name);
                parse_quote!(#path_ident)
            }
            [] => panic!("Empty generics_hierarchy"),
        }
    }
}

impl ItemDefinition {
    pub fn new(ast: &mut DeriveInput, nested: bool) -> syn::Result<Self> {
        let name = ast.ident.to_string();
        let data_struct = match &mut ast.data {
            Data::Struct(data_struct) => data_struct,
            _ => {
                return Err(syn::Error::new_spanned(
                    &ast.ident,
                    "#[aymond(...)] supports only structs",
                ));
            }
        };
        let fields_named = match &mut data_struct.fields {
            Fields::Named(fields_named) => fields_named,
            _ => {
                return Err(syn::Error::new_spanned(
                    &ast.ident,
                    "#[aymond(...)] supports only structs with named fields",
                ));
            }
        };

        let mut hash_key = None;
        let mut sort_key = None;
        let mut other_attributes = vec![];
        let mut gsis: HashMap<String, GsiDefinition> = HashMap::new();
        let mut lsis: HashMap<String, LsiDefinition> = HashMap::new();
        let mut version_attribute = None;
        let mut errors = None;

        for field in &mut fields_named.named {
            let aymond_attrs: Vec<_> = field
                .attrs
                .iter()
                .filter(|a| a.path().is_ident("aymond"))
                .collect();

            let mut is_hash = false;
            let mut is_sort = false;
            let mut custom_name = None;
            let mut is_version = false;
            let mut gsi_entries: Vec<(String, GsiRole)> = vec![];
            let mut lsi_entries: Vec<String> = vec![];
            let mut field_has_error = false;

            for attr in &aymond_attrs {
                let inner: Meta = match attr.parse_args() {
                    Ok(meta) => meta,
                    Err(err) => {
                        combine_error(
                            &mut errors,
                            syn::Error::new_spanned(
                                attr,
                                format!("invalid #[aymond(...)] field annotation: {err}"),
                            ),
                        );
                        field_has_error = true;
                        continue;
                    }
                };
                match &inner {
                    Meta::Path(p) if p.is_ident("hash_key") => is_hash = true,
                    Meta::Path(p) if p.is_ident("sort_key") => is_sort = true,
                    Meta::List(list) if list.path.is_ident("hash_key") => {
                        is_hash = true;
                        match Self::extract_attribute_name(list) {
                            Ok(name) => custom_name = name,
                            Err(err) => {
                                combine_error(&mut errors, err);
                                field_has_error = true;
                            }
                        }
                    }
                    Meta::List(list) if list.path.is_ident("sort_key") => {
                        is_sort = true;
                        match Self::extract_attribute_name(list) {
                            Ok(name) => custom_name = name,
                            Err(err) => {
                                combine_error(&mut errors, err);
                                field_has_error = true;
                            }
                        }
                    }
                    Meta::List(list) if list.path.is_ident("attribute") => {
                        match Self::extract_attribute_args(list) {
                            Ok((name, version)) => {
                                custom_name = name;
                                is_version = version;
                            }
                            Err(err) => {
                                combine_error(&mut errors, err);
                                field_has_error = true;
                            }
                        }
                    }
                    Meta::List(list) if list.path.is_ident("gsi") => {
                        match Self::parse_gsi_args(list) {
                            Ok(args) => gsi_entries.push(args),
                            Err(err) => {
                                combine_error(&mut errors, err);
                                field_has_error = true;
                            }
                        }
                    }
                    Meta::List(list) if list.path.is_ident("lsi") => {
                        match Self::parse_lsi_args(list) {
                            Ok(name) => lsi_entries.push(name),
                            Err(err) => {
                                combine_error(&mut errors, err);
                                field_has_error = true;
                            }
                        }
                    }
                    _ => {
                        combine_error(
                            &mut errors,
                            syn::Error::new_spanned(
                                &inner,
                                "unknown #[aymond(...)] field annotation; expected one of: \
                                 hash_key, sort_key, attribute(...), gsi(...), lsi(...)",
                            ),
                        );
                        field_has_error = true;
                    }
                }
            }

            if field_has_error {
                field
                    .attrs
                    .retain(|attr_def| !attr_def.path().is_ident("aymond"));
                continue;
            }

            if is_hash && hash_key.is_some() {
                combine_error(
                    &mut errors,
                    syn::Error::new_spanned(
                        &field.ident,
                        "multiple fields are marked with #[aymond(hash_key)]",
                    ),
                );
            } else if is_sort && sort_key.is_some() {
                combine_error(
                    &mut errors,
                    syn::Error::new_spanned(
                        &field.ident,
                        "multiple fields are marked with #[aymond(sort_key)]",
                    ),
                );
            }

            let Some(field_ident) = field.ident.clone() else {
                combine_error(
                    &mut errors,
                    syn::Error::new_spanned(field, "field must be named"),
                );
                continue;
            };
            let field_name = field_ident.to_string();
            let attr_name = custom_name.unwrap_or_else(|| field_name.clone());

            let ty = field.ty.clone();
            let item_attribute = ItemAttribute::new(field_ident, attr_name, ty);

            for (idx_name, role) in gsi_entries {
                let def = gsis
                    .entry(idx_name.clone())
                    .or_insert_with(|| GsiDefinition {
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

            if is_version {
                let numeric_types = [
                    "i8", "i16", "i32", "i64", "i128", "u8", "u16", "u32", "u64", "u128",
                ];
                if !numeric_types.contains(&item_attribute.generics_hierarchy[0].as_str()) {
                    combine_error(
                        &mut errors,
                        syn::Error::new_spanned(
                            &item_attribute.field,
                            "#[aymond(attribute(version))] field must be a numeric type",
                        ),
                    );
                }
                if item_attribute.is_option {
                    combine_error(
                        &mut errors,
                        syn::Error::new_spanned(
                            &item_attribute.field,
                            "#[aymond(attribute(version))] field cannot be Option",
                        ),
                    );
                }
                if version_attribute.is_some() {
                    combine_error(
                        &mut errors,
                        syn::Error::new_spanned(
                            &item_attribute.field,
                            "multiple fields are marked with #[aymond(attribute(version))]",
                        ),
                    );
                }
                version_attribute = Some(item_attribute.clone());
            }

            if is_hash {
                hash_key = Some(item_attribute);
            } else if is_sort {
                sort_key = Some(item_attribute);
            } else {
                other_attributes.push(item_attribute);
            }

            if hash_key.as_ref().filter(|e| e.is_option).is_some() {
                combine_error(
                    &mut errors,
                    syn::Error::new_spanned(
                        &field.ty,
                        "#[aymond(hash_key)] field cannot be Option<T>",
                    ),
                );
            } else if sort_key.as_ref().filter(|e| e.is_option).is_some() {
                combine_error(
                    &mut errors,
                    syn::Error::new_spanned(
                        &field.ty,
                        "#[aymond(sort_key)] field cannot be Option<T>",
                    ),
                );
            }

            field
                .attrs
                .retain(|attr_def| !attr_def.path().is_ident("aymond"));
        }

        if !nested && hash_key.is_none() {
            combine_error(
                &mut errors,
                syn::Error::new_spanned(
                    &ast.ident,
                    "#[aymond(hash_key)] is required for #[aymond(item)]",
                ),
            );
        }

        if let Some(err) = errors {
            return Err(err);
        }

        Ok(ItemDefinition {
            name,
            hash_key,
            sort_key,
            other_attributes,
            global_secondary_indexes: gsis,
            local_secondary_indexes: lsis,
            version_attribute,
        })
    }

    pub fn all_attributes(&self) -> impl Iterator<Item = &ItemAttribute> {
        self.hash_key
            .iter()
            .chain(self.sort_key.iter())
            .chain(self.other_attributes.iter())
    }

    fn parse_gsi_args(list: &MetaList) -> syn::Result<(String, GsiRole)> {
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
        let args: GsiArgs = list.parse_args().map_err(|err| {
            syn::Error::new_spanned(
                list,
                format!(
                    "invalid gsi annotation; expected: gsi(\"name\", hash_key | sort_key): {err}"
                ),
            )
        })?;
        let name = args.name.value();
        let role = if args.role.is_ident("hash_key") {
            GsiRole::HashKey
        } else if args.role.is_ident("sort_key") {
            GsiRole::SortKey
        } else {
            return Err(syn::Error::new_spanned(
                args.role,
                "invalid GSI role; expected hash_key or sort_key",
            ));
        };
        Ok((name, role))
    }

    fn parse_lsi_args(list: &MetaList) -> syn::Result<String> {
        let name: LitStr = list.parse_args().map_err(|err| {
            syn::Error::new_spanned(
                list,
                format!("invalid lsi annotation; expected: lsi(\"name\"): {err}"),
            )
        })?;
        Ok(name.value())
    }

    /// Parses `attribute(...)` args, returning `(custom_name, is_version)`.
    /// Supports: `attribute(name = "x")`, `attribute(version)`, `attribute(name = "x", version)`.
    fn extract_attribute_args(list: &MetaList) -> syn::Result<(Option<String>, bool)> {
        let mut custom_name = None;
        let mut is_version = false;

        let metas = list
            .parse_args_with(Punctuated::<Meta, Token![,]>::parse_terminated)
            .map_err(|err| {
                syn::Error::new_spanned(
                    list,
                    format!(
                        "invalid #[aymond(attribute(...))] syntax; expected \
                         attribute(name = \"...\") and/or attribute(version): {err}"
                    ),
                )
            })?;

        for meta in metas {
            match &meta {
                Meta::NameValue(nv) if nv.path.is_ident("name") => {
                    if let Expr::Lit(expr_lit) = &nv.value
                        && let Lit::Str(s) = &expr_lit.lit
                    {
                        custom_name = Some(s.value());
                    } else {
                        return Err(syn::Error::new_spanned(
                            nv,
                            "invalid attribute name value; expected name = \"...\"",
                        ));
                    }
                }
                Meta::Path(p) if p.is_ident("version") => {
                    is_version = true;
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        meta,
                        "unknown attribute argument; expected `name = \"...\"` or `version`",
                    ));
                }
            }
        }

        Ok((custom_name, is_version))
    }

    fn extract_attribute_name(list: &MetaList) -> syn::Result<Option<String>> {
        Ok(Self::extract_attribute_args(list)?.0)
    }
}
