use oxc_allocator::Allocator;
use oxc_ast::ast::{Expression, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::{ExtractOption, ExtractOutput, ExtractStyleValue};

mod assignments;
mod dependencies;
mod errors;
mod identity;
mod omissions;
mod values;

fn extract(body: &str) -> ExtractOutput {
    crate::extract(
        "/src/transitions.tsx",
        &format!("import stylex from '@stylexjs/stylex';\n{body}"),
        ExtractOption {
            single_css: true,
            import_main_css: false,
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("transition compilation: {error}"))
}

fn names(output: &ExtractOutput) -> Vec<String> {
    let allocator = Allocator::default();
    let program = Parser::new(&allocator, &output.code, SourceType::tsx())
        .parse()
        .program;
    program
        .body
        .iter()
        .filter_map(|statement| match statement {
            Statement::VariableDeclaration(declaration) => Some(declaration),
            _ => None,
        })
        .flat_map(|declaration| &declaration.declarations)
        .filter_map(|declaration| match &declaration.init {
            Some(Expression::StringLiteral(value)) => Some(value.value.to_string()),
            _ => None,
        })
        .collect()
}

fn css(output: &ExtractOutput) -> Vec<String> {
    let mut css: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Css(css) => Some(css.css.clone()),
            _ => None,
        })
        .collect();
    css.sort();
    css
}

fn reset() {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    css::set_prefix(None);
    css::debug::set_debug(false);
}
