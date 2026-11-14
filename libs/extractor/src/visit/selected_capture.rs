//! Native member selection carries a primitive branch tag and evaluated leaves.

use super::DevupVisitor;
use super::branch_capture::BranchReads;
use super::capture::Captured;
use oxc_allocator::{CloneIn, FromIn, GetAllocator, TakeIn};
use oxc_ast::ast::{
    ArrayExpressionElement, ChainElement, ComputedMemberExpression, Expression, ObjectPropertyKind,
    PropertyKey, PropertyKind, Str,
};
use oxc_ast_visit::VisitMut;
use oxc_span::SPAN;
use oxc_syntax::operator::UnaryOperator;
use rustc_hash::FxHashMap;

#[cfg(test)]
#[path = "selected_capture_tests.rs"]
mod tests;

/// Names every object inherits, which a key its table does not hold still reads.
pub(crate) const INHERITED: [&str; 12] = [
    "__defineGetter__",
    "__defineSetter__",
    "__lookupGetter__",
    "__lookupSetter__",
    "__proto__",
    "constructor",
    "hasOwnProperty",
    "isPrototypeOf",
    "propertyIsEnumerable",
    "toLocaleString",
    "toString",
    "valueOf",
];

/// A literal key into a table with no observable evaluation keeps its key for
/// the existing member extractor to prune: an own position selects its value,
/// and one the table neither holds nor inherits selects no style.
fn literal_own_selection(member: &oxc_ast::ast::ComputedMemberExpression<'_>) -> bool {
    if member.optional {
        return false;
    }
    let key = crate::utils::unwrap_syntax_only(&member.expression);
    let mut table = &member.object;
    loop {
        table = match table {
            Expression::ParenthesizedExpression(inner) => &inner.expression,
            Expression::TSAsExpression(inner) => &inner.expression,
            _ => break,
        };
    }
    let selects = match table {
        // An array inherits no number: one it does not hold reads `undefined`
        Expression::ArrayExpression(_) => array_index(key),
        Expression::ObjectExpression(object) => object_key(key).is_some_and(|key| {
            !INHERITED.contains(&key.as_str())
                || object
                    .properties
                    .iter()
                    .any(|property| own_key(property).is_some_and(|own| own == key))
        }),
        _ => false,
    };
    selects && evaluates_nothing(table)
}

/// `1`, `-1` or `+1`, which every array reads at the same place
fn array_index(key: &Expression<'_>) -> bool {
    let key = match key {
        Expression::UnaryExpression(unary)
            if matches!(
                unary.operator,
                UnaryOperator::UnaryNegation | UnaryOperator::UnaryPlus
            ) =>
        {
            crate::utils::unwrap_syntax_only(&unary.argument)
        }
        key => key,
    };
    matches!(key, Expression::NumericLiteral(_))
}

/// The property a literal key names, spelled as JavaScript spells it
fn object_key(key: &Expression<'_>) -> Option<String> {
    match key {
        Expression::StringLiteral(text) => text.value.as_str().map(str::to_string),
        Expression::TemplateLiteral(template) if template.expressions.is_empty() => template
            .quasis
            .first()
            .and_then(|quasi| quasi.value.cooked.as_ref())
            .and_then(|text| text.as_str().map(str::to_string)),
        Expression::NumericLiteral(number)
            if number.value.fract() == 0.0 && (0.0..=MAX_SAFE_INTEGER).contains(&number.value) =>
        {
            Some(number.value.to_string())
        }
        _ => None,
    }
}

const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;

/// The key a plain property of a table holds as its own
pub(crate) fn own_key(property: &ObjectPropertyKind<'_>) -> Option<String> {
    let ObjectPropertyKind::ObjectProperty(property) = property else {
        return None;
    };
    if property.kind != PropertyKind::Init || property.method || property.computed {
        return None;
    }
    let key = match &property.key {
        PropertyKey::StaticIdentifier(name) => name.name.as_str().to_string(),
        PropertyKey::StringLiteral(text) => text.value.as_str()?.to_string(),
        _ => return None,
    };
    // `__proto__:` written as is sets the prototype, not a property
    (key != "__proto__").then_some(key)
}

/// Whether reading the table evaluates nothing the code can observe
fn evaluates_nothing(table: &Expression<'_>) -> bool {
    let mut pending = vec![(table, true)];
    while let Some((value, top)) = pending.pop() {
        match value {
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    match element {
                        // A hole reads `undefined` only where it is selected
                        ArrayExpressionElement::Elision(_) if top => {}
                        element => match element.as_expression() {
                            Some(value) => pending.push((value, false)),
                            None => return false,
                        },
                    }
                }
            }
            Expression::ObjectExpression(object) if top => {
                for property in &object.properties {
                    let (Some(_), ObjectPropertyKind::ObjectProperty(property)) =
                        (own_key(property), property)
                    else {
                        return false;
                    };
                    pending.push((&property.value, false));
                }
            }
            Expression::StringLiteral(_)
            | Expression::NumericLiteral(_)
            | Expression::BooleanLiteral(_)
            | Expression::NullLiteral(_) => {}
            _ => return false,
        }
    }
    true
}

impl<'a> DevupVisitor<'a> {
    /// Known scalar alternatives remain static even when a spread makes the
    /// selected position unknown. Compare the captured value without coercion.
    pub(super) fn capture_spread_selection(
        &mut self,
        value: &mut Expression<'a>,
        captured: &mut Vec<Captured<'a>>,
    ) -> bool {
        let Expression::ComputedMemberExpression(member) = crate::utils::unwrap_syntax_only(value)
        else {
            return false;
        };
        let Expression::ArrayExpression(array) = crate::utils::unwrap_syntax_only(&member.object)
        else {
            return false;
        };
        if !array
            .elements
            .iter()
            .any(|element| matches!(element, ArrayExpressionElement::SpreadElement(_)))
        {
            return false;
        }
        let literals: Vec<_> = array
            .elements
            .iter()
            .filter_map(ArrayExpressionElement::as_expression)
            .map(crate::utils::unwrap_syntax_only)
            .filter(|value| {
                matches!(value, Expression::StringLiteral(string) if string.value.as_str().is_some())
                    || matches!(value, Expression::NumericLiteral(number) if number.value.is_finite())
            })
            .map(|value| value.clone_in(self.ast.allocator()))
            .collect();
        // Static classes must not require a new function around a field or default.
        if literals.is_empty() || !self.stylex_captures.binds() {
            return false;
        }
        let name = self.names.fresh("__devupValue");
        captured.push(self.capture_as(name, value));
        let read = value.clone_in(self.ast.allocator());
        for literal in literals.into_iter().rev() {
            *value = Expression::new_conditional_expression(
                SPAN,
                Expression::new_binary_expression(
                    SPAN,
                    read.clone_in(self.ast.allocator()),
                    oxc_syntax::operator::BinaryOperator::StrictEquality,
                    literal.clone_in(self.ast.allocator()),
                    &self.ast,
                ),
                literal,
                value.take_in(&self.ast),
                &self.ast,
            );
        }
        true
    }

    fn selected_slot(&self, name: &str, index: usize) -> Expression<'a> {
        // Keep the chain node so later member extraction preserves a missing selection.
        Expression::new_chain_expression(
            SPAN,
            ChainElement::ComputedMemberExpression(ComputedMemberExpression::boxed(
                SPAN,
                Expression::new_identifier(
                    SPAN,
                    Str::from_in(name, self.ast.allocator()),
                    &self.ast,
                ),
                Expression::new_string_literal(
                    SPAN,
                    Str::from_in(index.to_string().as_str(), self.ast.allocator()),
                    None,
                    &self.ast,
                ),
                true,
                &self.ast,
            )),
            &self.ast,
        )
    }

    fn selected_branch(
        &mut self,
        name: &str,
        key: &str,
        branch: &mut Expression<'a>,
    ) -> Expression<'a> {
        let mut captured = Vec::new();
        self.capture_shape(branch, &mut captured);
        let mut reads = BranchReads {
            ast: &self.ast,
            reads: FxHashMap::default(),
        };
        let mut values = oxc_allocator::Vec::new_in(&self.ast);
        values.push(
            Expression::new_string_literal(
                SPAN,
                Str::from_in(key, self.ast.allocator()),
                None,
                &self.ast,
            )
            .into(),
        );
        for (index, (leaf, value)) in captured.into_iter().enumerate() {
            reads
                .reads
                .insert(leaf, self.selected_slot(name, index + 1));
            values.push(value.into());
        }
        reads.visit_expression(branch);
        Expression::new_array_expression(SPAN, values, &self.ast)
    }

    pub(super) fn capture_selection(
        &mut self,
        member: &mut oxc_ast::ast::ComputedMemberExpression<'a>,
        captured: &mut Vec<Captured<'a>>,
    ) -> bool {
        if literal_own_selection(member) {
            return true;
        }
        let name = self.names.fresh("__devupChoice");
        let mut table = crate::utils::unwrap_syntax_only(&member.object)
            .clone_in_with_semantic_ids(self.ast.allocator());
        match &mut table {
            Expression::ArrayExpression(array) if array.elements.iter().all(|element|!matches!(element,ArrayExpressionElement::SpreadElement(_)))=>{
                let Expression::ArrayExpression(original)=crate::utils::unwrap_syntax_only_mut(&mut member.object)else{return false};
                for(index,(payload,branch))in array.elements.iter_mut().zip(&mut original.elements).enumerate(){
                    if let Some(branch)=branch.as_expression_mut(){
                        *payload=self.selected_branch(&name,&index.to_string(),branch).into();
                    }
                }
            }
            Expression::ObjectExpression(object) if object.properties.iter().all(|property|matches!(property,ObjectPropertyKind::ObjectProperty(property) if property.kind==PropertyKind::Init&&!property.method&&crate::utils::get_str_by_property_key(&property.key).is_some()))=>{
                let Expression::ObjectExpression(original)=crate::utils::unwrap_syntax_only_mut(&mut member.object)else{return false};
                for(payload,branch)in object.properties.iter_mut().zip(&mut original.properties){
                    if let(ObjectPropertyKind::ObjectProperty(payload),ObjectPropertyKind::ObjectProperty(branch))=(payload,branch)
                        && let Some(key)=crate::utils::get_str_by_property_key(&branch.key){
                        payload.value=self.selected_branch(&name,&key,&mut branch.value);
                        payload.shorthand=false;
                    }
                }
            }
            _=>return false,
        }
        let key = member.expression.take_in(&self.ast);
        member.expression = self.selected_slot(&name, 0);
        let selection = Expression::new_computed_member_expression(
            member.span,
            table,
            key,
            member.optional,
            &self.ast,
        );
        captured.push((name, selection));
        true
    }
}
