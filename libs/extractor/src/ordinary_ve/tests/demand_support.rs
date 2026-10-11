use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::{ExtractOutput, ExtractStyleValue, ResolvedModule, extract_with_modules};

pub(super) type TestResult = Result<(), Box<dyn std::error::Error>>;

pub(super) fn reset() {
    css::file_map::reset_file_map();
    css::class_map::reset_class_map();
}

pub(super) fn run(
    path: &str,
    source: &str,
    modules: &[(&str, &str, &str)],
) -> Result<ExtractOutput, Box<dyn std::error::Error>> {
    let modules: Vec<_> = modules
        .iter()
        .map(|(name, path, code)| {
            (
                (*name).to_string(),
                (*path).to_string(),
                (*code).to_string(),
            )
        })
        .collect();
    extract_with_modules(
        path,
        source,
        super::option(),
        false,
        &move |specifier, _| {
            modules.iter().find_map(|(name, path, code)| {
                (name == specifier).then(|| ResolvedModule {
                    path: path.clone(),
                    code: code.clone(),
                })
            })
        },
    )
}

pub(super) fn assert_import(output: &ExtractOutput, source: &str) {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &output.code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{}", output.code);
    assert!(
        parsed
            .program
            .body
            .iter()
            .any(|statement| matches!(statement,
        Statement::ImportDeclaration(import) if import.source.value.as_str() == source)),
        "missing import {source}: {}",
        output.code
    );
}

pub(super) fn static_value<'a>(output: &'a ExtractOutput, property: &str) -> &'a str {
    for style in &output.styles {
        if let ExtractStyleValue::Static(style) = style
            && style.property == property
        {
            return &style.value;
        }
    }
    panic!("missing {property}: {:?}", output.styles);
}

pub(super) fn global_selector(output: &ExtractOutput, property: &str) -> String {
    for style in &output.styles {
        if let ExtractStyleValue::Static(style) = style
            && style.property == property
            && let Some(css::style_selector::StyleSelector::Global(selector, _)) = &style.selector
        {
            return selector.clone();
        }
    }
    panic!("missing global {property}: {:?}", output.styles);
}
