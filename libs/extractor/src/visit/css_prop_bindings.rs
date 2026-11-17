//! Bindings Emotion's `css` prop reads, as `const rules = on ? { color: 'red' }
//! : { color: 'blue' }` with `<div css={rules} />`. Emotion reads the styles
//! the binding holds when the element renders. When only `css` props read the
//! binding, the styles it chooses among compile in its declaration, as if
//! each were written in `css()`, so the binding holds their classes and the
//! choice still runs where the code wrote it. Any other binding to styles the
//! build cannot read in the prop is reported where the prop reads it, never
//! passed on as a class.

use oxc_allocator::TakeIn;
use oxc_ast::ast::{
    Argument, ArrowFunctionBody, ArrowFunctionExpression, BindingPattern, Expression, Function,
    FunctionBody, JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXElement,
    JSXElementName, JSXExpression, LogicalOperator, Program, ReturnStatement, VariableDeclarator,
};
use oxc_ast_visit::{Visit, walk};
use oxc_span::SPAN;
use oxc_syntax::reference::ReferenceId;
use oxc_syntax::scope::ScopeFlags;
use oxc_syntax::symbol::SymbolId;
use rustc_hash::FxHashSet;

use super::DevupVisitor;
use crate::scope::NAMESPACE_CSS;
use crate::utils::{LOCAL_STYLES, unwrap_syntax_only, unwrap_syntax_only_mut};

/// A binding the `css` prop cannot read: it holds what a call to a function
/// of the module returning a style object gives, which the build does not run
pub(crate) const CALL_STYLES: &str = "it holds what a call gives, and the function returns a style object, which the build does not run the call to read: write the styles in the prop or in a `const` it reads, or return a class `css()` gives";

/// A binding the `css` prop cannot read: it chooses among styles, and code
/// other than `css` props reads it too
pub(crate) const CHOSEN_STYLES: &str = "it chooses among styles the build compiles only when `css` props alone read a `const` holding them: read it only as `css`, or write the choice in the prop";

/// What a value a binding chooses holds, as the `css` prop reads it
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Leaf {
    /// Styles: an object, an array, CSS text, or a binding to a local style
    /// object
    Styles,
    /// Nothing: `null`, `undefined` or a boolean
    Empty,
    /// What the prop composes as a class: a class `css()` gives, or a value
    /// only the runtime knows
    Class,
}

/// The references that are the whole value of a `css` prop an element takes
pub(crate) fn css_prop_reads<'a>(
    program: &Program<'a>,
    takes: &dyn Fn(&JSXElementName<'a>) -> bool,
) -> FxHashSet<ReferenceId> {
    let mut reads = Reads {
        takes,
        found: FxHashSet::default(),
    };
    reads.visit_program(program);
    reads.found
}

/// Collects the references that are the whole value of a `css` prop
struct Reads<'t, 'a> {
    takes: &'t dyn Fn(&JSXElementName<'a>) -> bool,
    found: FxHashSet<ReferenceId>,
}

impl<'a> Visit<'a> for Reads<'_, 'a> {
    fn visit_jsx_element(&mut self, it: &JSXElement<'a>) {
        if (self.takes)(&it.opening_element.name) {
            self.found.extend(
                it.opening_element
                    .attributes
                    .iter()
                    .filter_map(css_prop_reference),
            );
        }
        walk::walk_jsx_element(self, it);
    }
}

/// The binding `attribute` reads when it is `css={binding}`
fn css_prop_reference(attribute: &JSXAttributeItem<'_>) -> Option<ReferenceId> {
    let JSXAttributeItem::Attribute(attribute) = attribute else {
        return None;
    };
    let JSXAttributeName::Identifier(name) = &attribute.name else {
        return None;
    };
    let Some(JSXAttributeValue::ExpressionContainer(container)) = &attribute.value else {
        return None;
    };
    match (name.name.as_str(), &container.expression) {
        ("css", JSXExpression::Identifier(identifier)) => identifier.reference_id.get(),
        _ => None,
    }
}

/// Whether `init` chooses, with a condition or `&&`, `||` or `??`, among
/// values one of which `leaf` tells is styles
pub(crate) fn chooses_styles<'a>(
    init: &Expression<'a>,
    leaf: &dyn Fn(&Expression<'a>) -> Leaf,
) -> bool {
    match unwrap_syntax_only(init) {
        Expression::ConditionalExpression(conditional) => {
            holds_styles(&conditional.consequent, leaf)
                || holds_styles(&conditional.alternate, leaf)
        }
        Expression::LogicalExpression(logical) => {
            (logical.operator != LogicalOperator::And && holds_styles(&logical.left, leaf))
                || holds_styles(&logical.right, leaf)
        }
        _ => false,
    }
}

/// Whether `value`, or a value it chooses, is styles
fn holds_styles<'a>(value: &Expression<'a>, leaf: &dyn Fn(&Expression<'a>) -> Leaf) -> bool {
    chooses_styles(value, leaf) || leaf(value) == Leaf::Styles
}

/// `value` with each value it chooses that is styles compiled by `css()`, and,
/// when `empty` tells to, each that is nothing an empty class, so it holds
/// classes alone
fn compile_choice<'a>(
    ast: &oxc_ast::builder::AstBuilder<'a>,
    value: &mut Expression<'a>,
    leaf: &dyn Fn(&Expression<'a>) -> Leaf,
    empty: bool,
) {
    let value = unwrap_syntax_only_mut(value);
    match value {
        Expression::ConditionalExpression(conditional) => {
            compile_choice(ast, &mut conditional.consequent, leaf, empty);
            compile_choice(ast, &mut conditional.alternate, leaf, empty);
        }
        // `test && styles`: nothing while `test` is falsy, which as a class
        // would be the text of `false`, `0` or `''`
        Expression::LogicalExpression(logical) if logical.operator == LogicalOperator::And => {
            let test = logical.left.take_in(ast);
            let mut styles = logical.right.take_in(ast);
            compile_choice(ast, &mut styles, leaf, empty);
            *value = Expression::new_conditional_expression(
                SPAN,
                test,
                styles,
                Expression::new_string_literal(SPAN, "", None, ast),
                ast,
            );
        }
        // The left of `||` and `??` keeps what it holds when it is nothing,
        // which decides whether the right applies
        Expression::LogicalExpression(logical) => {
            compile_choice(ast, &mut logical.left, leaf, false);
            compile_choice(ast, &mut logical.right, leaf, empty);
        }
        value => match leaf(value) {
            Leaf::Styles => {
                let span = oxc_span::GetSpan::span(value);
                let styles = value.take_in(ast);
                *value = Expression::new_call_expression(
                    span,
                    Expression::new_identifier(SPAN, NAMESPACE_CSS, ast),
                    None::<oxc_allocator::Box<'a, oxc_ast::ast::TSTypeParameterInstantiation<'a>>>,
                    oxc_allocator::Vec::from_array_in([Argument::from(styles)], ast),
                    false,
                    ast,
                );
            }
            Leaf::Empty if empty => *value = Expression::new_string_literal(SPAN, "", None, ast),
            Leaf::Empty | Leaf::Class => {}
        },
    }
}

impl<'a> DevupVisitor<'a> {
    /// What `value` holds as a value a binding chooses
    pub(super) fn css_leaf(&self, value: &Expression<'a>) -> Leaf {
        match unwrap_syntax_only(value) {
            Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_) => Leaf::Styles,
            Expression::NullLiteral(_) | Expression::BooleanLiteral(_) => Leaf::Empty,
            Expression::Identifier(identifier) if identifier.name == "undefined" => Leaf::Empty,
            value if self.reads_local_styles(value) => Leaf::Styles,
            _ => Leaf::Class,
        }
    }

    /// Compile the styles `init` chooses among when `id` is a `const` only
    /// `css` props read; whether it did
    pub(super) fn compile_css_choice(
        &self,
        id: &BindingPattern<'a>,
        init: &mut Expression<'a>,
    ) -> bool {
        let compiles = self.style_values.constant(id).is_some_and(|symbol| {
            self.style_values.is_read(symbol)
                && self.style_values.read_only_by(symbol, &self.css_prop_reads)
        }) && chooses_styles(init, &|value| self.css_leaf(value));
        if compiles {
            compile_choice(&self.ast, init, &|value| self.css_leaf(value), true);
        }
        compiles
    }

    /// Why the `css` prop cannot read the binding `init` initializes as a
    /// class, when it cannot: it holds styles, or what a call to a function
    /// of the module returning styles gives
    pub(super) fn css_binding_requirement(&self, init: &Expression<'a>) -> Option<&'static str> {
        match unwrap_syntax_only(init) {
            Expression::ObjectExpression(_)
            | Expression::ArrayExpression(_)
            | Expression::ArrowFunctionExpression(_)
            | Expression::FunctionExpression(_)
            | Expression::StringLiteral(_)
            | Expression::TemplateLiteral(_) => Some(LOCAL_STYLES),
            Expression::CallExpression(call)
                if self
                    .style_values
                    .symbol(&call.callee)
                    .is_some_and(|symbol| self.style_functions.contains(&symbol)) =>
            {
                Some(CALL_STYLES)
            }
            init if chooses_styles(init, &|value| self.css_leaf(value)) => Some(CHOSEN_STYLES),
            _ => None,
        }
    }
}

/// The functions of a program returning a style object, an array or a choice
/// among them, by binding: a `function` declaration, or a binding to an arrow
/// or function expression
pub(crate) fn style_functions(program: &Program<'_>) -> FxHashSet<SymbolId> {
    #[derive(Default)]
    struct Functions(FxHashSet<SymbolId>);
    impl<'a> Visit<'a> for Functions {
        fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
            if let Some(id) = &it.id
                && it.body.as_ref().is_some_and(|body| returns_styles(body))
            {
                self.0.extend(id.symbol_id.get());
            }
            walk::walk_function(self, it, flags);
        }
        fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
            if it.init.as_ref().is_some_and(function_returns_styles)
                && let Some(id) = it.id.get_binding_identifier()
            {
                self.0.extend(id.symbol_id.get());
            }
            walk::walk_variable_declarator(self, it);
        }
    }
    let mut functions = Functions::default();
    functions.visit_program(program);
    functions.0
}

/// Whether `init` is an arrow or function expression returning styles
fn function_returns_styles(init: &Expression<'_>) -> bool {
    match unwrap_syntax_only(init) {
        Expression::ArrowFunctionExpression(arrow) => match &arrow.body {
            ArrowFunctionBody::FunctionBody(body) => returns_styles(body),
            body => body.as_expression().is_some_and(is_style_literal),
        },
        Expression::FunctionExpression(function) => function
            .body
            .as_ref()
            .is_some_and(|body| returns_styles(body)),
        _ => false,
    }
}

/// Whether a function with `body` returns a style object, an array or a
/// choice among them
fn returns_styles(body: &FunctionBody<'_>) -> bool {
    #[derive(Default)]
    struct Returns(bool);
    impl<'a> Visit<'a> for Returns {
        fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
            self.0 |= it.argument.as_ref().is_some_and(is_style_literal);
        }
        fn visit_function(&mut self, _: &Function<'a>, _: ScopeFlags) {}
        fn visit_arrow_function_expression(&mut self, _: &ArrowFunctionExpression<'a>) {}
    }
    let mut returns = Returns::default();
    returns.visit_function_body(body);
    returns.0
}

/// Whether `value` is a style object, an array or a choice among them
fn is_style_literal(value: &Expression<'_>) -> bool {
    holds_styles(value, &|value| match unwrap_syntax_only(value) {
        Expression::ObjectExpression(_) | Expression::ArrayExpression(_) => Leaf::Styles,
        _ => Leaf::Class,
    })
}

#[cfg(test)]
mod tests {
    use oxc_allocator::Allocator;
    use oxc_parser::Parser;
    use oxc_semantic::SemanticBuilder;
    use oxc_span::SourceType;

    /// Only `css={binding}` on an element that takes the prop is a read:
    /// spreads, namespaced names, text, other expressions and other props are
    /// not
    #[test]
    fn css_prop_reads_only_whole_bindings() {
        let allocator = Allocator::default();
        let code = "const a = { b: 1 }, b = 2;\n<><div {...a} xlink:href={a} css=\"color: red\" title={a} /><div css={a.b} /><div css={a} /><span css={b} /></>;";
        let program = Parser::new(&allocator, code, SourceType::tsx())
            .parse()
            .program;
        let scoping = SemanticBuilder::new()
            .build(&program)
            .semantic
            .into_scoping();
        let reads = super::css_prop_reads(
            &program,
            &|name| !matches!(name, oxc_ast::ast::JSXElementName::Identifier(name) if name.name == "span"),
        );
        let read: Vec<_> = reads
            .iter()
            .map(|reference| scoping.get_reference(*reference).symbol_id())
            .collect();
        assert_eq!(read, [scoping.get_root_binding("a".into())]);
    }
}
