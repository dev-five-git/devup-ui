//! `StyleX` markers and the `when.*()` conditions that read them, compiled to
//! the classes and `:where(...)` selectors `StyleX` gives them.

use super::DevupVisitor;
use crate::stylex::{DEFAULT_MARKER, StylexFunction, define_marker_class, when_condition};
use crate::utils::{
    build_time_error, get_string_by_literal_expression, readable_argument, unwrap_syntax_only,
};
use oxc_allocator::{FromIn, GetAllocator};
use oxc_ast::ast::{Argument, CallExpression, Expression, Str, VariableDeclarator};
use oxc_span::SPAN;
use std::borrow::Cow;

/// What a `when.*()` condition takes
const WHEN_ARGUMENTS: &str = "it takes a pseudo-class or an attribute selector as a string literal, and optionally a marker: `stylex.defaultMarker()`, a `stylex.defineMarker()` export of this file or imported, or a class name";

impl<'a> DevupVisitor<'a> {
    /// The class of the marker `symbol` binds, when it binds one
    pub(super) fn stylex_marker(&self, symbol: oxc_semantic::SymbolId) -> Option<&String> {
        self.stylex_marker_classes.get(&symbol)
    }

    fn string_literal(&self, text: &str) -> Expression<'a> {
        Expression::new_string_literal(
            SPAN,
            Str::from_in(text, self.ast.allocator()),
            None,
            &self.ast,
        )
    }

    /// `export const marker = stylex.defineMarker()`: the call becomes the
    /// class of the marker, named after the file and the binding
    pub(super) fn compile_define_marker(&mut self, it: &mut VariableDeclarator<'a>) {
        let Some(Expression::CallExpression(call)) = &it.init else {
            return;
        };
        if !call.arguments.is_empty()
            || !self.is_stylex_call(&call.callee, &StylexFunction::DefineMarker)
        {
            return;
        }
        let Some((id, symbol)) = it
            .id
            .get_binding_identifier()
            .and_then(|id| Some((id, id.symbol_id.get()?)))
        else {
            return;
        };
        let exported = self
            .exports
            .of(&id.name)
            .iter()
            .any(|name| name != "default");
        if !exported || !self.bindings.is_module_binding(symbol) {
            return;
        }
        let class = define_marker_class(&self.filename, &id.name, self.split_filename.as_deref());
        it.init = Some(self.string_literal(&class));
        self.stylex_marker_classes.insert(symbol, class);
    }

    /// `stylex.defaultMarker()` and `when.*()` calls become the class and the
    /// condition key they give; a `when.*()` call the build cannot read is a
    /// build error
    pub(super) fn compile_stylex_markers(&mut self, it: &mut Expression<'a>) {
        let Expression::CallExpression(call) = &*it else {
            return;
        };
        let compiled = if self.is_stylex_call(&call.callee, &StylexFunction::DefaultMarker) {
            if !call.arguments.is_empty() {
                return;
            }
            Cow::Borrowed(DEFAULT_MARKER)
        } else {
            let Some(relation) = self.when_relation(call) else {
                return;
            };
            match self.when_call(relation, call) {
                Ok(condition) => Cow::Owned(condition),
                Err(error) => {
                    let arguments: Vec<String> =
                        call.arguments.iter().map(readable_argument).collect();
                    self.errors.push((
                        call.span.start,
                        build_time_error(
                            &format!("stylex.when.{relation}"),
                            &arguments.join(", "),
                            error,
                        ),
                    ));
                    return;
                }
            }
        };
        *it = self.string_literal(&compiled);
    }

    /// The relation a `when.*()` call names, as `ancestor`
    fn when_relation<'c>(&self, call: &'c CallExpression<'a>) -> Option<&'c str> {
        let Expression::StaticMemberExpression(member) = &call.callee else {
            return None;
        };
        (self.bindings.stylex_function(&member.object) == Some(StylexFunction::When))
            .then_some(member.property.name.as_str())
    }

    fn when_call(&self, relation: &str, call: &CallExpression<'a>) -> Result<String, &'static str> {
        let (pseudo, marker) = match call.arguments.as_slice() {
            [pseudo] => (pseudo, None),
            [pseudo, marker] => (pseudo, Some(marker)),
            _ => return Err(WHEN_ARGUMENTS),
        };
        let pseudo = when_string(pseudo).ok_or(WHEN_ARGUMENTS)?;
        let marker = match marker {
            Some(marker) => self.marker_class(marker).ok_or(WHEN_ARGUMENTS)?,
            None => DEFAULT_MARKER.to_string(),
        };
        when_condition(relation, &pseudo, &marker)
    }

    /// The class of the marker a `when.*()` call takes: a class name, as
    /// `stylex.defaultMarker()` gives, or a `stylex.defineMarker()` binding
    fn marker_class(&self, marker: &Argument<'a>) -> Option<String> {
        if let Some(class) = when_string(marker) {
            return Some(class);
        }
        let Expression::Identifier(identifier) = unwrap_syntax_only(marker.as_expression()?) else {
            return None;
        };
        self.stylex_marker(self.bindings.symbol(identifier)?)
            .cloned()
    }
}

fn when_string(argument: &Argument<'_>) -> Option<String> {
    get_string_by_literal_expression(unwrap_syntax_only(argument.as_expression()?))
        .map(Cow::into_owned)
}
