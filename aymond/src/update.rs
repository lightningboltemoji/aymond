use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;

use aws_sdk_dynamodb::types::AttributeValue;

/// A segment of a DynamoDB document path for update expressions.
#[derive(Clone)]
pub enum PathSegment {
    /// Map key — gets an expression attribute name (#uN).
    Attr(String),
    /// List index — literal [N] in the expression.
    Index(usize),
}

enum UpdateAction {
    Set {
        path: Vec<PathSegment>,
        value: AttributeValue,
    },
    Add {
        path: Vec<PathSegment>,
        value: AttributeValue,
    },
    Remove {
        path: Vec<PathSegment>,
    },
    Delete {
        path: Vec<PathSegment>,
        value: AttributeValue,
    },
}

pub struct UpdateExpr {
    actions: Vec<UpdateAction>,
}

impl UpdateExpr {
    pub fn set(path: Vec<PathSegment>, value: AttributeValue) -> Self {
        Self {
            actions: vec![UpdateAction::Set { path, value }],
        }
    }

    pub fn add(path: Vec<PathSegment>, value: AttributeValue) -> Self {
        Self {
            actions: vec![UpdateAction::Add { path, value }],
        }
    }

    pub fn remove(path: Vec<PathSegment>) -> Self {
        Self {
            actions: vec![UpdateAction::Remove { path }],
        }
    }

    pub fn delete(path: Vec<PathSegment>, value: AttributeValue) -> Self {
        Self {
            actions: vec![UpdateAction::Delete { path, value }],
        }
    }

    pub fn and(mut self, mut other: UpdateExpr) -> UpdateExpr {
        self.actions.append(&mut other.actions);
        self
    }

    pub fn build(
        self,
    ) -> (
        String,
        HashMap<String, String>,
        HashMap<String, AttributeValue>,
    ) {
        let mut names = HashMap::new();
        let mut values = HashMap::new();
        let mut counter: usize = 0;
        let mut set_parts = Vec::new();
        let mut remove_parts = Vec::new();
        let mut delete_parts = Vec::new();

        for action in self.actions {
            match action {
                UpdateAction::Set { path, value } => {
                    let path_str = Self::render_path(&path, &mut counter, &mut names);
                    let value_ph = format!(":u{}", counter);
                    counter += 1;
                    values.insert(value_ph.clone(), value);
                    set_parts.push(format!("{} = {}", path_str, value_ph));
                }
                UpdateAction::Add { path, value } => {
                    let path_str = Self::render_path(&path, &mut counter, &mut names);
                    let value_ph = format!(":u{}", counter);
                    counter += 1;
                    values.insert(value_ph.clone(), value);
                    set_parts.push(format!("{} = {} + {}", path_str, path_str, value_ph));
                }
                UpdateAction::Remove { path } => {
                    let path_str = Self::render_path(&path, &mut counter, &mut names);
                    remove_parts.push(path_str);
                }
                UpdateAction::Delete { path, value } => {
                    let path_str = Self::render_path(&path, &mut counter, &mut names);
                    let value_ph = format!(":u{}", counter);
                    counter += 1;
                    values.insert(value_ph.clone(), value);
                    delete_parts.push(format!("{} {}", path_str, value_ph));
                }
            }
        }

        let mut parts = Vec::new();
        if !set_parts.is_empty() {
            parts.push(format!("SET {}", set_parts.join(", ")));
        }
        if !remove_parts.is_empty() {
            parts.push(format!("REMOVE {}", remove_parts.join(", ")));
        }
        if !delete_parts.is_empty() {
            parts.push(format!("DELETE {}", delete_parts.join(", ")));
        }
        (parts.join(" "), names, values)
    }

    fn render_path(
        path: &[PathSegment],
        counter: &mut usize,
        names: &mut HashMap<String, String>,
    ) -> String {
        let mut parts = Vec::new();
        for seg in path {
            match seg {
                PathSegment::Attr(name) => {
                    let placeholder = format!("#u{}", *counter);
                    *counter += 1;
                    names.insert(placeholder.clone(), name.clone());
                    parts.push(placeholder);
                }
                PathSegment::Index(i) => {
                    if let Some(last) = parts.last_mut() {
                        *last = format!("{}[{}]", last, i);
                    }
                }
            }
        }
        parts.join(".")
    }
}

pub trait IntoUpdateValue {
    fn into_update_value(self) -> AttributeValue;
}

pub trait IntoUpdateNumberValue {
    fn into_update_number_value(self) -> AttributeValue;
}

pub trait IntoUpdateSetValue {
    fn into_update_set_value(values: HashSet<Self>) -> AttributeValue
    where
        Self: Sized;
}

impl IntoUpdateValue for String {
    fn into_update_value(self) -> AttributeValue {
        AttributeValue::S(self)
    }
}

impl IntoUpdateValue for &str {
    fn into_update_value(self) -> AttributeValue {
        AttributeValue::S(self.to_string())
    }
}

impl IntoUpdateValue for bool {
    fn into_update_value(self) -> AttributeValue {
        AttributeValue::Bool(self)
    }
}

impl IntoUpdateValue for Vec<u8> {
    fn into_update_value(self) -> AttributeValue {
        AttributeValue::B(aws_sdk_dynamodb::primitives::Blob::new(self))
    }
}

impl IntoUpdateValue for HashSet<String> {
    fn into_update_value(self) -> AttributeValue {
        AttributeValue::Ss(self.into_iter().collect())
    }
}

impl IntoUpdateValue for HashSet<Vec<u8>> {
    fn into_update_value(self) -> AttributeValue {
        AttributeValue::Bs(
            self.into_iter()
                .map(aws_sdk_dynamodb::primitives::Blob::new)
                .collect(),
        )
    }
}

impl IntoUpdateSetValue for String {
    fn into_update_set_value(values: HashSet<Self>) -> AttributeValue {
        AttributeValue::Ss(values.into_iter().collect())
    }
}

impl IntoUpdateSetValue for Vec<u8> {
    fn into_update_set_value(values: HashSet<Self>) -> AttributeValue {
        AttributeValue::Bs(
            values
                .into_iter()
                .map(aws_sdk_dynamodb::primitives::Blob::new)
                .collect(),
        )
    }
}

macro_rules! impl_into_update_numeric {
    ($($t:ty),+) => {
        $(
            impl IntoUpdateValue for $t {
                fn into_update_value(self) -> AttributeValue {
                    AttributeValue::N(self.to_string())
                }
            }

            impl IntoUpdateNumberValue for $t {
                fn into_update_number_value(self) -> AttributeValue {
                    AttributeValue::N(self.to_string())
                }
            }
        )+
    };
}

impl_into_update_numeric!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

pub trait UpdatePathRoot: Sized {
    fn with_prefix(path: Vec<PathSegment>) -> Self;
}

pub struct ScalarUpdatePath<T: IntoUpdateValue> {
    path: Vec<PathSegment>,
    _phantom: PhantomData<T>,
}

impl<T: IntoUpdateValue> UpdatePathRoot for ScalarUpdatePath<T> {
    fn with_prefix(path: Vec<PathSegment>) -> Self {
        Self {
            path,
            _phantom: PhantomData,
        }
    }
}

impl<T: IntoUpdateValue> ScalarUpdatePath<T> {
    pub fn set(self, v: impl Into<T>) -> UpdateExpr {
        UpdateExpr::set(self.path, v.into().into_update_value())
    }
}

impl<T: IntoUpdateNumberValue + IntoUpdateValue> ScalarUpdatePath<T> {
    pub fn add(self, v: impl Into<T>) -> UpdateExpr {
        UpdateExpr::add(self.path, v.into().into_update_number_value())
    }
}

pub struct ListUpdatePath<T: UpdatePathRoot> {
    path_prefix: Vec<PathSegment>,
    _phantom: PhantomData<T>,
}

impl<T: UpdatePathRoot> UpdatePathRoot for ListUpdatePath<T> {
    fn with_prefix(path: Vec<PathSegment>) -> Self {
        Self {
            path_prefix: path,
            _phantom: PhantomData,
        }
    }
}

impl<T: UpdatePathRoot> ListUpdatePath<T> {
    pub fn index(&self, i: usize) -> T {
        let mut path = self.path_prefix.clone();
        path.push(PathSegment::Index(i));
        T::with_prefix(path)
    }
}

pub struct SetUpdatePath<T: IntoUpdateSetValue + Eq + Hash> {
    path: Vec<PathSegment>,
    _phantom: PhantomData<T>,
}

impl<T: IntoUpdateSetValue + Eq + Hash> UpdatePathRoot for SetUpdatePath<T> {
    fn with_prefix(path: Vec<PathSegment>) -> Self {
        Self {
            path,
            _phantom: PhantomData,
        }
    }
}

impl<T: IntoUpdateSetValue + Eq + Hash> SetUpdatePath<T> {
    pub fn set(self, values: HashSet<T>) -> UpdateExpr {
        UpdateExpr::set(self.path, T::into_update_set_value(values))
    }

    pub fn delete(self, v: impl Into<T>) -> UpdateExpr {
        let mut values = HashSet::new();
        values.insert(v.into());
        UpdateExpr::delete(self.path, T::into_update_set_value(values))
    }

    pub fn delete_set(self, values: HashSet<T>) -> UpdateExpr {
        UpdateExpr::delete(self.path, T::into_update_set_value(values))
    }
}

pub trait IntoOptionalUpdateExpr {
    fn into_optional_update_expr(self) -> Option<UpdateExpr>;
}

impl IntoOptionalUpdateExpr for UpdateExpr {
    fn into_optional_update_expr(self) -> Option<UpdateExpr> {
        Some(self)
    }
}

impl IntoOptionalUpdateExpr for () {
    fn into_optional_update_expr(self) -> Option<UpdateExpr> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{PathSegment, UpdateExpr};
    use aws_sdk_dynamodb::types::AttributeValue;

    #[test]
    fn test_update_expr_render_set_remove_and_delete() {
        let expr = UpdateExpr::add(
            vec![PathSegment::Attr("count".into())],
            AttributeValue::N("10".into()),
        )
        .and(UpdateExpr::remove(vec![PathSegment::Attr("flag".into())]))
        .and(UpdateExpr::delete(
            vec![PathSegment::Attr("labels".into())],
            AttributeValue::Ss(vec!["rust".into()]),
        ));
        let (rendered, _, _) = expr.build();
        assert_eq!(rendered, "SET #u0 = #u0 + :u1 REMOVE #u2 DELETE #u3 :u4");
    }
}
