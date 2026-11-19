//! Units for style values only known at runtime.
//!
//! A literal number is converted at build time (`p={2}` → `padding:8px`), but
//! a value read at runtime reached its CSS variable as written, and `2` is no
//! CSS length. While a file is compiled, every such value is passed through a
//! helper the file declares once, converting a number the way the build
//! converts a literal of that property.

use std::cell::RefCell;

use oxc_allocator::{FromIn, GetAllocator};
use oxc_ast::{
    ast::{Argument, Expression, NumberBase, Str},
    builder::AstBuilder,
};
use oxc_span::SPAN;

use crate::extract_style::constant::{
    TIME_PROPERTIES, is_maintain_value_property, strip_vendor_prefix,
};

thread_local! {
    /// The name of the helper the file being compiled calls, while it scales
    /// numbers read at runtime
    static HELPER: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Whether a style's value is converted where it is set: what the value is
/// for, not which value it is, so it never tells two styles apart
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Marked(pub(crate) bool);

impl PartialEq for Marked {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}

impl Eq for Marked {}

impl PartialOrd for Marked {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Marked {
    fn cmp(&self, _: &Self) -> std::cmp::Ordering {
        std::cmp::Ordering::Equal
    }
}

impl std::hash::Hash for Marked {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {}
}

/// The base of the helper's name, made unique in each file
pub(crate) const HELPER_BASE: &str = "__devupScale";

/// Lets values read at runtime be converted while compiling one file, and
/// restores the outer file's helper when it ends
pub(crate) struct Guard(Option<String>);

impl Guard {
    pub(crate) fn new(helper: Option<String>) -> Self {
        Self(HELPER.with(|cell| cell.replace(helper)))
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        let outer = self.0.take();
        HELPER.with(|cell| *cell.borrow_mut() = outer);
    }
}

/// The helper's name, when the file being compiled converts runtime numbers
fn helper() -> Option<String> {
    HELPER.with(|cell| cell.borrow().clone())
}

/// Whether the file being compiled converts numbers read at runtime with the
/// helper, which then leaves no other conversion to make
pub(crate) fn active() -> bool {
    HELPER.with(|cell| cell.borrow().is_some())
}

/// What a number of a property becomes, as a literal of it is converted
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Unit {
    /// Four pixels a unit
    Length,
    /// Milliseconds
    Time,
}

/// The unit a number of `property` takes, or `None` when the number is kept as
/// written, or when no length is valid for the property, a color
pub(crate) fn unit(property: &str) -> Option<Unit> {
    let unprefixed = strip_vendor_prefix(property);
    if property.starts_with("--")
        || is_maintain_value_property(property)
        || unprefixed.ends_with("color")
    {
        None
    } else if TIME_PROPERTIES.contains(unprefixed) {
        Some(Unit::Time)
    } else {
        Some(Unit::Length)
    }
}

/// Whether `code` may hold a number at runtime: anything but a template
/// literal ending in text no number ends in, like `` `${a}px` ``
pub(crate) fn may_be_number(code: &str) -> bool {
    !(code.len() > 1
        && code.starts_with('`')
        && code.ends_with('`')
        && code[..code.len() - 1]
            .chars()
            .next_back()
            .is_some_and(|last| !last.is_ascii_digit() && !matches!(last, '.' | '}' | '`')))
}

/// The unit a number `property` reads at runtime is converted to, in a file
/// converting such numbers
pub(crate) fn runtime_unit(property: &str) -> Option<Unit> {
    if active() { unit(property) } else { None }
}

/// `value` passed through the file's helper to convert a number to `unit`, as
/// a literal is converted
pub(crate) fn wrap<'a>(
    builder: &AstBuilder<'a>,
    unit: Option<Unit>,
    value: Expression<'a>,
) -> Expression<'a> {
    let (Some(unit), Some(helper)) = (unit, helper()) else {
        return value;
    };
    let mut arguments = oxc_allocator::Vec::from_array_in([Argument::from(value)], builder);
    if unit == Unit::Time {
        arguments.push(Argument::from(Expression::new_numeric_literal(
            SPAN,
            1.0,
            None,
            NumberBase::Decimal,
            builder,
        )));
        arguments.push(Argument::from(Expression::new_string_literal(
            SPAN, "ms", None, builder,
        )));
    }
    Expression::new_call_expression(
        SPAN,
        Expression::new_identifier(SPAN, Str::from_in(&helper, builder.allocator()), builder),
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterInstantiation<'a>>>,
        arguments,
        false,
        builder,
    )
}

/// The value `value` converts, when it is a call of the file's helper, which
/// reads only that value and observes nothing else
pub(crate) fn converted<'s, 'a>(value: &'s Expression<'a>) -> &'s Expression<'a> {
    match value {
        Expression::CallExpression(call)
            if matches!(&call.callee, Expression::Identifier(callee)
                if helper().is_some_and(|helper| callee.name == helper.as_str())) =>
        {
            call.arguments[0].to_expression()
        }
        _ => value,
    }
}

/// The helper `name` declares: a number, or a string holding only one, as
/// Devup UI reads a literal, times `s` in the unit `u`; any other value as it
/// is, so `null`, `undefined` and `false` still remove the declaration. It
/// converts no object to text, so it runs no code of the value's own
pub(crate) fn declaration(name: &str) -> String {
    format!(
        "function {name}(v, s = 4, u = \"px\") {{\n\treturn typeof v === \"number\" || typeof v === \"string\" && /^[+-]?(\\d+\\.?\\d*|\\.\\d+)(e[+-]?\\d+)?$/i.test(v) ? v * s + u : v;\n}}\n"
    )
}

/// Whether `source` is compiled with Devup UI's units: no element takes
/// Emotion's `css` prop, and neither the file nor a stylesheet reads a library
/// whose numbers are pixels
pub(crate) fn applies(
    source: &str,
    filename: &str,
    aliases: &std::collections::HashMap<String, crate::ImportAlias>,
    library_rules: bool,
) -> bool {
    !library_rules
        && !crate::utils::is_vanilla_extract_file(filename)
        && !aliases
            .keys()
            .any(|package| source.contains(package.as_str()))
}

/// `code` with the helper declared at its end, when it calls it; a function
/// declaration is hoisted, so every call before it already reads it
pub(crate) fn declare(code: &mut String, name: &str) {
    if code.contains(&format!("{name}(")) {
        code.push_str(&declaration(name));
    }
}

#[cfg(test)]
#[path = "runtime_units_tests.rs"]
mod tests;
