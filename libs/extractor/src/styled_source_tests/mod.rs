mod compatibility;
mod execution;
mod lowering;

use crate::{ExtractOption, extract, utils::readable_code};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use oxc_allocator::Allocator;
use oxc_ast::ast::{
    ArrowFunctionExpression, BindingPattern, CallExpression, Expression,
    ImportDeclarationSpecifier, ObjectPropertyKind, Program, Statement,
};
use oxc_ast_visit::{
    Visit,
    walk::{walk_arrow_function_expression, walk_call_expression},
};
use oxc_parser::Parser;
use oxc_span::SourceType;
use oxc_syntax::reference::ReferenceId;
use rstest::rstest;
use serial_test::serial;

const IMPORT: &str = "import { styled, Box } from '@devup-ui/react';\n";

fn parse<'a>(arena: &'a Allocator, filename: &str, code: &'a str) -> Program<'a> {
    let source_type = match SourceType::from_path(filename) {
        Ok(source_type) => source_type,
        Err(error) => panic!("{filename}: {error}"),
    };
    let parsed = Parser::new(arena, code, source_type).parse();
    assert_eq!(
        parsed.diagnostics.len(),
        0,
        "{filename}\n{code}\n{:?}",
        parsed.diagnostics
    );
    assert!(!parsed.fatal_error, "{filename}\n{code}");
    parsed.program
}

fn compile(filename: &str, source: &str, single_css: bool) -> String {
    let arena = Allocator::default();
    parse(&arena, filename, source);
    reset_class_map();
    reset_file_map();
    match extract(
        filename,
        source,
        ExtractOption {
            single_css,
            import_main_css: false,
            ..ExtractOption::default()
        },
    ) {
        Ok(output) => output.code,
        Err(error) => panic!("{filename}\n{source}\n{error}"),
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Target {
    Tag(String),
    Component(String),
    Member(String),
}

#[derive(Debug)]
struct Render {
    helper: String,
    reference: Option<ReferenceId>,
    target: Target,
    props: Vec<(String, String)>,
}

#[derive(Default)]
struct Renders(Vec<Render>);

impl<'a> Visit<'a> for Renders {
    fn visit_arrow_function_expression(&mut self, arrow: &ArrowFunctionExpression<'a>) {
        for parameter in &arrow.params.items {
            let pattern = match &parameter.pattern {
                BindingPattern::ObjectPattern(pattern) => pattern,
                _ => continue,
            };
            let keys: Vec<_> = pattern
                .properties
                .iter()
                .filter_map(|property| property.key.static_name())
                .collect();
            if keys.iter().any(|key| key.as_ref() == "style")
                && keys.iter().any(|key| key.as_ref() == "className")
            {
                assert_eq!(
                    keys.iter()
                        .map(std::convert::AsRef::as_ref)
                        .collect::<Vec<_>>(),
                    ["style", "className"]
                );
                assert_eq!(
                    pattern
                        .rest
                        .as_ref()
                        .and_then(|rest| rest.argument.get_binding_identifier())
                        .map(|id| id.name.as_str()),
                    Some("rest")
                );
            }
        }
        walk_arrow_function_expression(self, arrow);
    }

    fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
        walk_call_expression(self, call);
        let (target, props) = match call.arguments.as_slice() {
            [target, props] => (target, props),
            _ => return,
        };
        let object = match props.as_expression() {
            Some(Expression::ObjectExpression(object)) => object,
            _ => return,
        };
        let props: Vec<_> = object
            .properties
            .iter()
            .map(|property| match property {
                ObjectPropertyKind::ObjectProperty(property) => (
                    property
                        .key
                        .static_name()
                        .map_or_else(String::new, |name| name.to_string()),
                    readable_code(&property.value),
                ),
                ObjectPropertyKind::SpreadProperty(spread) => {
                    ("...".into(), readable_code(&spread.argument))
                }
            })
            .collect();
        if !props
            .iter()
            .any(|(key, value)| key == "..." && value == "rest")
            || !props.iter().any(|(key, _)| key == "className")
            || !props.iter().any(|(key, _)| key == "style")
        {
            return;
        }
        let target = match target.as_expression() {
            Some(Expression::StringLiteral(tag)) => Target::Tag(tag.value.to_string()),
            Some(Expression::Identifier(component)) => {
                Target::Component(component.name.to_string())
            }
            Some(expression @ Expression::StaticMemberExpression(_)) => {
                Target::Member(readable_code(expression))
            }
            other => panic!("unexpected render target: {other:?}"),
        };
        let (helper, reference) = match &call.callee {
            Expression::Identifier(helper) => (helper.name.to_string(), helper.reference_id.get()),
            other => panic!("unexpected render helper: {other:?}"),
        };
        self.0.push(Render {
            helper,
            reference,
            target,
            props,
        });
    }
}

fn renders(program: &Program<'_>) -> Vec<Render> {
    let mut calls = Renders::default();
    calls.visit_program(program);
    calls.0
}

fn jsx_imports(program: &Program<'_>) -> Vec<String> {
    program
        .body
        .iter()
        .flat_map(|statement| match statement {
            Statement::ImportDeclaration(import) if import.source.value == "react/jsx-runtime" => {
                import
                    .specifiers
                    .iter()
                    .flatten()
                    .filter_map(|specifier| match specifier {
                        ImportDeclarationSpecifier::ImportSpecifier(named)
                            if named.imported.to_string() == "jsx"
                                && !import.import_kind.is_type()
                                && !named.import_kind.is_type() =>
                        {
                            Some(named.local.name.to_string())
                        }
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            }
            _ => vec![],
        })
        .collect()
}
