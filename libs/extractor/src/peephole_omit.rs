//! `Object.fromEntries(Object.entries(rest).filter(([k]) => ...))`, which each
//! styled component writes to drop the props it does not pass on, written
//! once per module as a function the components call when more than one
//! drops the same props.

use oxc_allocator::{CloneIn, GetAllocator, TakeIn, Vec};
use oxc_ast::{
    ast::{Argument, BindingPattern, CallExpression, Expression, Program, Statement},
    builder::AstBuilder,
};
use oxc_ast_visit::{Visit, VisitMut, walk, walk_mut};
use oxc_span::{ContentEq, SPAN};
use rustc_hash::FxHashSet;

use super::{unwrapped, unwrapped_mut};

/// The function each repeated filter is called through
const OMIT: &str = "__devupOmit";

/// Write each filter more than one component writes alike as a call of one
/// function of the module
pub(super) fn share<'a>(
    ast: &AstBuilder<'a>,
    program: &mut Program<'a>,
    declared: &FxHashSet<String>,
) {
    // The function reads `Object` where the module does.
    if declared.contains("Object") {
        return;
    }
    let mut found = Found::default();
    found.visit_program(program);
    let shared = found
        .predicates
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(predicate, _)| predicate)
        .collect::<std::vec::Vec<_>>();
    if shared.is_empty() {
        return;
    }
    let mut index = 0;
    let names = shared
        .iter()
        .map(|_| {
            loop {
                let name = format!("{OMIT}{index}");
                index += 1;
                if !declared.contains(&name) {
                    break name;
                }
            }
        })
        .collect::<std::vec::Vec<_>>();
    let mut predicates = std::vec::Vec::new();
    let mut functions = std::vec::Vec::new();
    for (predicate, name) in shared.iter().zip(&names) {
        let function = format!(
            "function {name}(__devupOmitted) {{ return Object.fromEntries(Object.entries(__devupOmitted).filter({predicate})); }}"
        );
        if let Ok(predicate) = parse(ast, predicate).parse_expression()
            && let Some(function) = parse(ast, &function)
                .parse()
                .program
                .body
                .into_iter()
                .next()
        {
            predicates.push((predicate, name));
            functions.push(function);
        }
    }
    Calls {
        ast,
        predicates: predicates
            .iter()
            .map(|(predicate, name)| (predicate, *name))
            .collect(),
    }
    .visit_program(program);
    // Written after the calls are, the functions keep the filter they share.
    let at = program
        .body
        .iter()
        .take_while(|statement| matches!(statement, Statement::ImportDeclaration(_)))
        .count();
    for function in functions.into_iter().rev() {
        program.body.insert(at, function);
    }
}

/// The value `call` filters and the predicate it filters with, when `call`
/// is `Object.fromEntries(Object.entries(value).filter(predicate))` of a
/// predicate reading nothing but its key
pub(super) fn filtered<'s, 'a>(
    call: &'s CallExpression<'a>,
) -> Option<(&'s Expression<'a>, &'s Expression<'a>)> {
    let object_member = |callee: &Expression<'_>, property: &str| {
        matches!(unwrapped(callee), Expression::StaticMemberExpression(member)
            if !member.optional
                && member.property.name == property
                && matches!(unwrapped(&member.object), Expression::Identifier(id) if id.name == "Object"))
    };
    if call.optional || !object_member(&call.callee, "fromEntries") {
        return None;
    }
    let [Argument::CallExpression(filter)] = call.arguments.as_slice() else {
        return None;
    };
    let Expression::StaticMemberExpression(member) = unwrapped(&filter.callee) else {
        return None;
    };
    let [predicate] = filter.arguments.as_slice() else {
        return None;
    };
    let Expression::CallExpression(entries) = unwrapped(&member.object) else {
        return None;
    };
    let [value] = entries.arguments.as_slice() else {
        return None;
    };
    if filter.optional
        || member.optional
        || member.property.name != "filter"
        || entries.optional
        || !object_member(&entries.callee, "entries")
    {
        return None;
    }
    let (Some(value), Some(predicate)) = (value.as_expression(), predicate.as_expression()) else {
        return None;
    };
    let Expression::ArrowFunctionExpression(arrow) = unwrapped(predicate) else {
        return None;
    };
    let [parameter] = arrow.params.items.as_slice() else {
        return None;
    };
    let BindingPattern::ArrayPattern(pattern) = &parameter.pattern else {
        return None;
    };
    let [Some(BindingPattern::BindingIdentifier(key))] = pattern.elements.as_slice() else {
        return None;
    };
    if arrow.r#async
        || arrow.params.rest.is_some()
        || pattern.rest.is_some()
        || parameter.initializer.is_some()
    {
        return None;
    }
    // Read where the module begins, the predicate reads the same names.
    let mut reads = Reads::default();
    reads.visit_arrow_function_expression(arrow);
    reads
        .names
        .iter()
        .all(|name| name == key.name.as_str())
        .then_some((value, predicate))
}

/// The names `predicate` reads
#[derive(Default)]
struct Reads {
    names: FxHashSet<String>,
}

impl<'a> Visit<'a> for Reads {
    fn visit_identifier_reference(&mut self, it: &oxc_ast::ast::IdentifierReference<'a>) {
        self.names.insert(it.name.to_string());
    }

    fn visit_this_expression(&mut self, _: &oxc_ast::ast::ThisExpression) {
        self.names.insert("this".to_string());
    }
}

/// Each predicate the module filters with, written as code, and how often
#[derive(Default)]
struct Found {
    predicates: std::vec::Vec<(String, usize)>,
}

impl<'a> Visit<'a> for Found {
    fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
        if let Some((_, predicate)) = filtered(it) {
            let code = code(predicate);
            match self.predicates.iter_mut().find(|(seen, _)| *seen == code) {
                Some((_, count)) => *count += 1,
                None => self.predicates.push((code, 1)),
            }
        }
        walk::walk_call_expression(self, it);
    }
}

/// Call each shared filter through its function
struct Calls<'s, 'a> {
    ast: &'s AstBuilder<'a>,
    predicates: std::vec::Vec<(&'s Expression<'a>, &'s String)>,
}

impl<'a> VisitMut<'a> for Calls<'_, 'a> {
    fn visit_expression(&mut self, it: &mut Expression<'a>) {
        walk_mut::walk_expression(self, it);
        let Expression::CallExpression(call) = it else {
            return;
        };
        let Some((_, predicate)) = filtered(call) else {
            return;
        };
        let Some((_, name)) = self
            .predicates
            .iter()
            .find(|(shared, _)| shared.content_eq(unwrapped(predicate)))
        else {
            return;
        };
        let name: &'a str = self.ast.allocator().alloc_str(name);
        if let Some(Argument::CallExpression(filter)) = call.arguments.first_mut()
            && let Expression::StaticMemberExpression(member) = unwrapped_mut(&mut filter.callee)
            && let Expression::CallExpression(entries) = unwrapped_mut(&mut member.object)
        {
            let value = entries.arguments.take_in(self.ast);
            *it = Expression::new_call_expression(
                SPAN,
                Expression::new_identifier(SPAN, name, self.ast),
                None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterInstantiation<'a>>>,
                value,
                false,
                self.ast,
            );
        }
    }
}

fn code(expression: &Expression<'_>) -> String {
    let allocator = oxc_allocator::Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let statement =
        Statement::new_expression_statement(SPAN, unwrapped(expression).clone_in(&allocator), &ast);
    let program = Program::new(
        SPAN,
        oxc_span::SourceType::mjs(),
        "",
        Vec::new_in(&ast),
        None,
        Vec::new_in(&ast),
        Vec::from_array_in([statement], &ast),
        &ast,
    );
    let code = oxc_codegen::Codegen::new().build(&program).code;
    code.trim_end().trim_end_matches(';').to_string()
}

fn parse<'a>(ast: &AstBuilder<'a>, code: &str) -> oxc_parser::Parser<'a> {
    oxc_parser::Parser::new(
        ast.allocator(),
        ast.allocator().alloc_str(code),
        oxc_span::SourceType::mjs(),
    )
}
