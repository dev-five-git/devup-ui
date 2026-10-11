use oxc_allocator::{Allocator, Box, CloneIn};
use oxc_ast::ast::{
    CallExpression, ComputedMemberExpression, ConditionalExpression, Expression,
    IdentifierReference, StaticMemberExpression, StringLiteral, TemplateLiteral,
};

pub(crate) mod products;

/// An owned local result whose root was supplied by a finished constructor.
pub(crate) enum FinishedClass<'a> {
    String(Box<'a, StringLiteral<'a>>),
    Template(Box<'a, TemplateLiteral<'a>>),
    Call(Box<'a, CallExpression<'a>>),
    Conditional(Box<'a, ConditionalExpression<'a>>),
}

/// A local body whose class leaves require the caller's actual capture bundle.
pub(crate) enum CapturedClassBody<'a> {
    String(Box<'a, StringLiteral<'a>>),
    Template(Box<'a, TemplateLiteral<'a>>),
    Call(Box<'a, CallExpression<'a>>),
    Conditional(Box<'a, ConditionalExpression<'a>>),
    Identifier(Box<'a, IdentifierReference<'a>>),
    StaticMember(Box<'a, StaticMemberExpression<'a>>),
    ComputedMember(Box<'a, ComputedMemberExpression<'a>>),
}

pub(crate) trait ClassPayload<'a>: Sized {
    fn clone_payload(&self, alloc: &'a Allocator) -> Self;
    fn clone_expression(&self, alloc: &'a Allocator) -> Expression<'a>;
    fn into_expression(self) -> Expression<'a>;
    fn string_value(&self) -> Option<&str>;
}

pub(crate) trait ClassConstructors<'a>: ClassPayload<'a> {
    fn from_string(value: Box<'a, StringLiteral<'a>>) -> Self;
    fn from_template(value: Box<'a, TemplateLiteral<'a>>) -> Self;
}

pub(crate) trait ClassConditional<'a>: ClassConstructors<'a> {
    fn from_conditional(value: Box<'a, ConditionalExpression<'a>>) -> Self;
}

pub(crate) trait ClassMergeRoot<'a>: Sized {
    fn merge_string_value(&self) -> Option<&str>;
    fn into_merge_expression(self) -> Expression<'a>;
    fn merge_string(value: Box<'a, StringLiteral<'a>>) -> Self;
    fn merge_template(value: Box<'a, TemplateLiteral<'a>>) -> Self;
}

impl<'a> ClassPayload<'a> for Expression<'a> {
    fn clone_payload(&self, alloc: &'a Allocator) -> Self {
        self.clone_in(alloc)
    }

    fn clone_expression(&self, alloc: &'a Allocator) -> Expression<'a> {
        self.clone_in(alloc)
    }

    fn into_expression(self) -> Expression<'a> {
        self
    }

    fn string_value(&self) -> Option<&str> {
        match self {
            Self::StringLiteral(value) => Some(value.value.as_str()),
            _ => None,
        }
    }
}

impl<'a> ClassConstructors<'a> for Expression<'a> {
    fn from_string(value: Box<'a, StringLiteral<'a>>) -> Self {
        Self::StringLiteral(value)
    }

    fn from_template(value: Box<'a, TemplateLiteral<'a>>) -> Self {
        Self::TemplateLiteral(value)
    }
}

macro_rules! local_payload {
    ($name:ident, $($variant:ident => $expression:ident),+ $(,)?) => {
        impl<'a> ClassPayload<'a> for $name<'a> {
            fn clone_payload(&self, alloc: &'a Allocator) -> Self {
                match self {
                    Self::String(value) => Self::String(value.clone_in(alloc)),
                    $(Self::$variant(value) => Self::$variant(value.clone_in(alloc)),)+
                }
            }

            fn clone_expression(&self, alloc: &'a Allocator) -> Expression<'a> {
                self.clone_payload(alloc).into_expression()
            }

            fn into_expression(self) -> Expression<'a> {
                match self {
                    Self::String(value) => Expression::StringLiteral(value),
                    $(Self::$variant(value) => Expression::$expression(value),)+
                }
            }

            fn string_value(&self) -> Option<&str> {
                match self {
                    Self::String(value) => Some(value.value.as_str()),
                    $(Self::$variant(_))|+ => None,
                }
            }
        }

        impl<'a> ClassConstructors<'a> for $name<'a> {
            fn from_string(value: Box<'a, StringLiteral<'a>>) -> Self {
                Self::String(value)
            }

            fn from_template(value: Box<'a, TemplateLiteral<'a>>) -> Self {
                Self::Template(value)
            }
        }

        impl<'a> ClassConditional<'a> for $name<'a> {
            fn from_conditional(value: Box<'a, ConditionalExpression<'a>>) -> Self {
                Self::Conditional(value)
            }
        }
    };
}

local_payload!(FinishedClass,
    Template => TemplateLiteral,
    Call => CallExpression,
    Conditional => ConditionalExpression,
);
local_payload!(CapturedClassBody,
    Template => TemplateLiteral,
    Call => CallExpression,
    Conditional => ConditionalExpression,
    Identifier => Identifier,
    StaticMember => StaticMemberExpression,
    ComputedMember => ComputedMemberExpression,
);

impl<'a, E: ClassConstructors<'a>> ClassMergeRoot<'a> for E {
    fn merge_string_value(&self) -> Option<&str> {
        self.string_value()
    }

    fn into_merge_expression(self) -> Expression<'a> {
        self.into_expression()
    }

    fn merge_string(value: Box<'a, StringLiteral<'a>>) -> Self {
        Self::from_string(value)
    }

    fn merge_template(value: Box<'a, TemplateLiteral<'a>>) -> Self {
        Self::from_template(value)
    }
}
