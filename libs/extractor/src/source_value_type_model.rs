use std::collections::BTreeMap;

use oxc_span::Span;
use oxc_syntax::{
    operator::{BinaryOperator, UnaryOperator},
    symbol::SymbolId,
};
use rustc_hash::FxHashMap;

use super::ValueType;

/// `BigInt` also carries a union's possible `BigInt` through arithmetic.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum Scalar {
    Number,
    String,
    NonNumericString,
    BigInt,
    Unproven,
}

#[derive(Clone, Debug)]
pub(super) enum Node {
    Scalar(Scalar),
    Symbol(SymbolId),
    Expression(Span),
    Object(BTreeMap<String, Self>),
    Union(Vec<Self>),
    Merge(Vec<Self>),
    Function(Box<Self>),
    Call(Box<Self>),
    Member(Box<Self>, String),
    Binary(BinaryOperator, Box<Self>, Box<Self>),
    Unary(UnaryOperator, Box<Self>),
    Primitive(Box<Self>),
    Import {
        source: String,
        export: Option<String>,
    },
}

impl Node {
    pub(super) const UNKNOWN: Self = Self::Scalar(Scalar::Unproven);

    pub(super) fn member(self, key: String) -> Self {
        Self::Member(Box::new(self), key)
    }
}

#[derive(Default)]
pub(super) struct Model {
    pub(super) bound_references: FxHashMap<u32, String>,
    pub(super) bindings: FxHashMap<SymbolId, Node>,
    pub(super) expressions: BTreeMap<Span, Node>,
    pub(super) exports: BTreeMap<String, Node>,
    pub(super) stars: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum Shape {
    Scalar(Scalar),
    Object(BTreeMap<String, Self>),
    Function(Box<Self>),
}

impl Shape {
    pub(super) const UNKNOWN: Self = Self::Scalar(Scalar::Unproven);

    pub(super) const fn value_type(&self) -> ValueType {
        match self {
            Self::Scalar(Scalar::Number) => ValueType::Number,
            Self::Scalar(Scalar::String) => ValueType::String,
            Self::Scalar(Scalar::NonNumericString) => ValueType::NonNumericString,
            Self::Scalar(Scalar::BigInt | Scalar::Unproven)
            | Self::Object(_)
            | Self::Function(_) => ValueType::Unproven,
        }
    }

    pub(super) fn join(self, other: Self) -> Self {
        if self == other {
            return self;
        }
        match (self, other) {
            (Self::Scalar(Scalar::String), Self::Scalar(Scalar::NonNumericString))
            | (Self::Scalar(Scalar::NonNumericString), Self::Scalar(Scalar::String)) => {
                Self::Scalar(Scalar::String)
            }
            (Self::Scalar(Scalar::BigInt), _) | (_, Self::Scalar(Scalar::BigInt)) => {
                Self::Scalar(Scalar::BigInt)
            }
            (Self::Object(left), Self::Object(right)) => Self::Object(
                left.into_iter()
                    .map(|(key, value)| {
                        let joined = value.join(right.get(&key).cloned().unwrap_or(Self::UNKNOWN));
                        (key, joined)
                    })
                    .collect(),
            ),
            (Self::Function(left), Self::Function(right)) => {
                Self::Function(Box::new(left.join(*right)))
            }
            (Self::Scalar(_) | Self::Object(_) | Self::Function(_), _) => Self::UNKNOWN,
        }
    }

    pub(super) fn member(self, key: &str) -> Self {
        match self {
            Self::Object(mut fields) => fields.remove(key).unwrap_or(Self::UNKNOWN),
            Self::Scalar(_) | Self::Function(_) => Self::UNKNOWN,
        }
    }
}
