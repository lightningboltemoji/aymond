use std::collections::HashMap;
use std::marker::PhantomData;

use aws_sdk_dynamodb::types::AttributeValue;

/// A segment of a DynamoDB document path.
#[derive(Clone)]
pub enum PathSegment {
    /// Map key — gets an expression attribute name (#nN).
    Attr(String),
    /// List index — literal [N] in the expression.
    Index(usize),
}

/// Expression tree for DynamoDB condition expressions.
pub enum CondExpr {
    Comparison {
        path: Vec<PathSegment>,
        op: &'static str,
        value: AttributeValue,
    },
    And(Box<CondExpr>, Box<CondExpr>),
    Or(Box<CondExpr>, Box<CondExpr>),
    Not(Box<CondExpr>),
    Contains {
        path: Vec<PathSegment>,
        value: AttributeValue,
    },
    BeginsWith {
        path: Vec<PathSegment>,
        value: AttributeValue,
    },
    Between {
        path: Vec<PathSegment>,
        low: AttributeValue,
        high: AttributeValue,
    },
}

impl CondExpr {
    pub fn and(self, other: CondExpr) -> CondExpr {
        CondExpr::And(Box::new(self), Box::new(other))
    }

    pub fn or(self, other: CondExpr) -> CondExpr {
        CondExpr::Or(Box::new(self), Box::new(other))
    }

    pub fn not(self) -> CondExpr {
        CondExpr::Not(Box::new(self))
    }

    /// Renders the expression tree into a condition expression string,
    /// expression attribute names, and expression attribute values.
    pub fn build(self) -> (String, HashMap<String, String>, HashMap<String, AttributeValue>) {
        let mut names = HashMap::new();
        let mut values = HashMap::new();
        let mut counter: usize = 0;
        let expr = self.render(&mut counter, &mut names, &mut values);
        (expr, names, values)
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
                    let placeholder = format!("#n{}", *counter);
                    *counter += 1;
                    names.insert(placeholder.clone(), name.clone());
                    parts.push(placeholder);
                }
                PathSegment::Index(i) => {
                    // Index appends to the previous part: #n0[0]
                    if let Some(last) = parts.last_mut() {
                        *last = format!("{}[{}]", last, i);
                    }
                }
            }
        }
        parts.join(".")
    }

    fn render(
        &self,
        counter: &mut usize,
        names: &mut HashMap<String, String>,
        values: &mut HashMap<String, AttributeValue>,
    ) -> String {
        match self {
            CondExpr::Comparison { path, op, value } => {
                let path_str = Self::render_path(path, counter, names);
                let val_placeholder = format!(":v{}", *counter);
                *counter += 1;
                values.insert(val_placeholder.clone(), value.clone());
                format!("{} {} {}", path_str, op, val_placeholder)
            }
            CondExpr::And(left, right) => {
                let l = left.render(counter, names, values);
                let r = right.render(counter, names, values);
                format!("({} AND {})", l, r)
            }
            CondExpr::Or(left, right) => {
                let l = left.render(counter, names, values);
                let r = right.render(counter, names, values);
                format!("({} OR {})", l, r)
            }
            CondExpr::Not(inner) => {
                let i = inner.render(counter, names, values);
                format!("(NOT {})", i)
            }
            CondExpr::Contains { path, value } => {
                let path_str = Self::render_path(path, counter, names);
                let val_placeholder = format!(":v{}", *counter);
                *counter += 1;
                values.insert(val_placeholder.clone(), value.clone());
                format!("contains({}, {})", path_str, val_placeholder)
            }
            CondExpr::BeginsWith { path, value } => {
                let path_str = Self::render_path(path, counter, names);
                let val_placeholder = format!(":v{}", *counter);
                *counter += 1;
                values.insert(val_placeholder.clone(), value.clone());
                format!("begins_with({}, {})", path_str, val_placeholder)
            }
            CondExpr::Between { path, low, high } => {
                let path_str = Self::render_path(path, counter, names);
                let low_placeholder = format!(":v{}", *counter);
                *counter += 1;
                values.insert(low_placeholder.clone(), low.clone());
                let high_placeholder = format!(":v{}", *counter);
                *counter += 1;
                values.insert(high_placeholder.clone(), high.clone());
                format!("{} BETWEEN {} AND {}", path_str, low_placeholder, high_placeholder)
            }
        }
    }
}

// ── IntoConditionValue ──

/// Converts a Rust value into a DynamoDB `AttributeValue` for use in condition expressions.
pub trait IntoConditionValue {
    fn into_condition_value(self) -> AttributeValue;
}

impl IntoConditionValue for String {
    fn into_condition_value(self) -> AttributeValue {
        AttributeValue::S(self)
    }
}

impl IntoConditionValue for &str {
    fn into_condition_value(self) -> AttributeValue {
        AttributeValue::S(self.to_string())
    }
}

impl IntoConditionValue for bool {
    fn into_condition_value(self) -> AttributeValue {
        AttributeValue::Bool(self)
    }
}

impl IntoConditionValue for Vec<u8> {
    fn into_condition_value(self) -> AttributeValue {
        AttributeValue::B(aws_sdk_dynamodb::primitives::Blob::new(self))
    }
}

macro_rules! impl_into_condition_value_numeric {
    ($($t:ty),+) => {
        $(
            impl IntoConditionValue for $t {
                fn into_condition_value(self) -> AttributeValue {
                    AttributeValue::N(self.to_string())
                }
            }
        )+
    };
}

impl_into_condition_value_numeric!(i8, i16, i32, i64, i128, u8, u16, u32, u64, u128);

// ── ConditionPathRoot ──

/// Trait for condition path types that can be constructed with a path prefix.
pub trait ConditionPathRoot: Sized {
    fn with_prefix(path: Vec<PathSegment>) -> Self;
}

// ── ScalarConditionPath ──

/// A typed path to a scalar DynamoDB attribute. Provides comparison methods.
pub struct ScalarConditionPath<T: IntoConditionValue> {
    path: Vec<PathSegment>,
    _phantom: PhantomData<T>,
}

impl<T: IntoConditionValue> ConditionPathRoot for ScalarConditionPath<T> {
    fn with_prefix(path: Vec<PathSegment>) -> Self {
        Self {
            path,
            _phantom: PhantomData,
        }
    }
}

impl<T: IntoConditionValue> ScalarConditionPath<T> {
    pub fn eq(self, v: impl Into<T>) -> CondExpr {
        CondExpr::Comparison {
            path: self.path,
            op: "=",
            value: v.into().into_condition_value(),
        }
    }

    pub fn ne(self, v: impl Into<T>) -> CondExpr {
        CondExpr::Comparison {
            path: self.path,
            op: "<>",
            value: v.into().into_condition_value(),
        }
    }

    pub fn lt(self, v: impl Into<T>) -> CondExpr {
        CondExpr::Comparison {
            path: self.path,
            op: "<",
            value: v.into().into_condition_value(),
        }
    }

    pub fn gt(self, v: impl Into<T>) -> CondExpr {
        CondExpr::Comparison {
            path: self.path,
            op: ">",
            value: v.into().into_condition_value(),
        }
    }

    pub fn le(self, v: impl Into<T>) -> CondExpr {
        CondExpr::Comparison {
            path: self.path,
            op: "<=",
            value: v.into().into_condition_value(),
        }
    }

    pub fn ge(self, v: impl Into<T>) -> CondExpr {
        CondExpr::Comparison {
            path: self.path,
            op: ">=",
            value: v.into().into_condition_value(),
        }
    }

    pub fn between(self, low: impl Into<T>, high: impl Into<T>) -> CondExpr {
        CondExpr::Between {
            path: self.path,
            low: low.into().into_condition_value(),
            high: high.into().into_condition_value(),
        }
    }
}

// begins_with for String paths
impl ScalarConditionPath<String> {
    pub fn begins_with(self, v: impl Into<String>) -> CondExpr {
        CondExpr::BeginsWith {
            path: self.path,
            value: AttributeValue::S(v.into()),
        }
    }
}

// begins_with for binary paths
impl ScalarConditionPath<Vec<u8>> {
    pub fn begins_with(self, v: Vec<u8>) -> CondExpr {
        CondExpr::BeginsWith {
            path: self.path,
            value: AttributeValue::B(aws_sdk_dynamodb::primitives::Blob::new(v)),
        }
    }
}

// ── ListConditionPath ──

/// A typed path to a DynamoDB list (L) attribute. Use `.index(N)` to access elements.
pub struct ListConditionPath<T: ConditionPathRoot> {
    path_prefix: Vec<PathSegment>,
    _phantom: PhantomData<T>,
}

impl<T: ConditionPathRoot> ConditionPathRoot for ListConditionPath<T> {
    fn with_prefix(path: Vec<PathSegment>) -> Self {
        Self {
            path_prefix: path,
            _phantom: PhantomData,
        }
    }
}

impl<T: ConditionPathRoot> ListConditionPath<T> {
    pub fn index(&self, i: usize) -> T {
        let mut path = self.path_prefix.clone();
        path.push(PathSegment::Index(i));
        T::with_prefix(path)
    }
}

// ── StringSetConditionPath ──

/// A typed path to a DynamoDB string set (SS) attribute.
pub struct StringSetConditionPath {
    path: Vec<PathSegment>,
}

impl ConditionPathRoot for StringSetConditionPath {
    fn with_prefix(path: Vec<PathSegment>) -> Self {
        Self { path }
    }
}

impl StringSetConditionPath {
    pub fn contains(self, v: impl Into<String>) -> CondExpr {
        CondExpr::Contains {
            path: self.path,
            value: AttributeValue::S(v.into()),
        }
    }
}

// ── BinarySetConditionPath ──

/// A typed path to a DynamoDB binary set (BS) attribute.
pub struct BinarySetConditionPath {
    path: Vec<PathSegment>,
}

impl ConditionPathRoot for BinarySetConditionPath {
    fn with_prefix(path: Vec<PathSegment>) -> Self {
        Self { path }
    }
}

impl BinarySetConditionPath {
    pub fn contains(self, v: Vec<u8>) -> CondExpr {
        CondExpr::Contains {
            path: self.path,
            value: AttributeValue::B(aws_sdk_dynamodb::primitives::Blob::new(v)),
        }
    }
}
