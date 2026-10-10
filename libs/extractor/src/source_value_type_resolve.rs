use std::collections::{BTreeMap, BTreeSet};

use oxc_span::Span;
use oxc_syntax::{
    operator::{BinaryOperator, UnaryOperator},
    symbol::SymbolId,
};

use super::{
    graph::Graph,
    model::{Model, Node, Scalar, Shape},
};

pub(super) struct Evaluation<'m> {
    model: &'m Model,
    filename: &'m str,
    symbols: BTreeSet<SymbolId>,
    expressions: BTreeSet<Span>,
    depth: usize,
}

impl<'m> Evaluation<'m> {
    pub(super) const fn new(model: &'m Model, filename: &'m str) -> Self {
        Self {
            model,
            filename,
            symbols: BTreeSet::new(),
            expressions: BTreeSet::new(),
            depth: 0,
        }
    }

    pub(super) fn resolve(&mut self, graph: &mut Graph<'_>, node: &Node) -> Shape {
        if self.depth >= 64 {
            return Shape::UNKNOWN;
        }
        self.depth += 1;
        let shape = self.node(graph, node);
        self.depth -= 1;
        shape
    }

    fn node(&mut self, graph: &mut Graph<'_>, node: &Node) -> Shape {
        match node {
            Node::Scalar(scalar) => Shape::Scalar(*scalar),
            Node::Symbol(symbol) => {
                if !self.symbols.insert(*symbol) {
                    return Shape::UNKNOWN;
                }
                let model = self.model;
                let shape = model
                    .bindings
                    .get(symbol)
                    .map_or(Shape::UNKNOWN, |node| self.resolve(graph, node));
                self.symbols.remove(symbol);
                shape
            }
            Node::Expression(span) => {
                if !self.expressions.insert(*span) {
                    return Shape::UNKNOWN;
                }
                let model = self.model;
                let shape = model
                    .expressions
                    .get(span)
                    .map_or(Shape::UNKNOWN, |node| self.resolve(graph, node));
                self.expressions.remove(span);
                shape
            }
            Node::Object(fields) => Shape::Object(
                fields
                    .iter()
                    .map(|(key, node)| (key.clone(), self.resolve(graph, node)))
                    .collect(),
            ),
            Node::Union(nodes) => nodes
                .iter()
                .map(|node| self.resolve(graph, node))
                .reduce(Shape::join)
                .unwrap_or(Shape::UNKNOWN),
            Node::Merge(nodes) => {
                let mut fields = BTreeMap::new();
                for node in nodes {
                    let Shape::Object(object) = self.resolve(graph, node) else {
                        return Shape::UNKNOWN;
                    };
                    for (key, value) in object {
                        fields
                            .entry(key)
                            .and_modify(|previous: &mut Shape| {
                                *previous = previous.clone().join(value.clone());
                            })
                            .or_insert(value);
                    }
                }
                Shape::Object(fields)
            }
            Node::Function(result) => Shape::Function(Box::new(self.resolve(graph, result))),
            Node::Call(callee) => match self.resolve(graph, callee) {
                Shape::Function(result) => *result,
                Shape::Scalar(_) | Shape::Object(_) => Shape::UNKNOWN,
            },
            Node::Member(object, key) => self.resolve(graph, object).member(key),
            Node::Primitive(value) => match self.resolve(graph, value) {
                value @ (Shape::Scalar(_) | Shape::Function(_)) => value,
                Shape::Object(_) => Shape::UNKNOWN,
            },
            Node::Import { source, export } => {
                let fields = graph.imported(source, self.filename);
                let object = Shape::Object(fields);
                match export {
                    Some(key) => object.member(key),
                    None => object,
                }
            }
            Node::Binary(operator, left, right) => binary(
                *operator,
                self.resolve(graph, left),
                self.resolve(graph, right),
            ),
            Node::Unary(operator, value) => unary(*operator, self.resolve(graph, value)),
        }
    }
}

fn binary(operator: BinaryOperator, left: Shape, right: Shape) -> Shape {
    use Scalar::{BigInt, NonNumericString, Number, String};
    match operator {
        BinaryOperator::Addition => match (left, right) {
            (Shape::Scalar(Number), Shape::Scalar(Number)) => Shape::Scalar(Number),
            (Shape::Scalar(String | NonNumericString), _)
            | (_, Shape::Scalar(String | NonNumericString)) => Shape::Scalar(String),
            (Shape::Scalar(BigInt), _) | (_, Shape::Scalar(BigInt)) => Shape::Scalar(BigInt),
            _ => Shape::UNKNOWN,
        },
        BinaryOperator::Subtraction
        | BinaryOperator::Multiplication
        | BinaryOperator::Division
        | BinaryOperator::Remainder => match (left, right) {
            (Shape::Scalar(BigInt), _) | (_, Shape::Scalar(BigInt)) => Shape::Scalar(BigInt),
            _ => Shape::Scalar(Number),
        },
        _ => Shape::UNKNOWN,
    }
}

fn unary(operator: UnaryOperator, value: Shape) -> Shape {
    match operator {
        UnaryOperator::UnaryPlus | UnaryOperator::UnaryNegation => match value {
            Shape::Scalar(Scalar::BigInt) => Shape::Scalar(Scalar::BigInt),
            Shape::Scalar(_) | Shape::Object(_) | Shape::Function(_) => {
                Shape::Scalar(Scalar::Number)
            }
        },
        UnaryOperator::Typeof => Shape::Scalar(Scalar::NonNumericString),
        _ => Shape::UNKNOWN,
    }
}
