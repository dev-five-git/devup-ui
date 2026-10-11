use oxc_allocator::Allocator;
use oxc_ast::ast::{BindingPattern, Declaration, Expression, Statement};
use oxc_parser::Parser;
use oxc_span::SourceType;

use super::demand_support::assert_import;
use super::mixed_support::{assert_consumed, has_static};
use crate::{ExtractOutput, ExtractStyleValue};

pub(super) const DATA_PATH: &str = "/initializer-key-data.ts";
pub(super) const CONSUMER: &str = "import {style} from '@vanilla-extract/css';import {palette} from './data';export const box=style({margin:palette.space});";
pub(super) const CONSTANT_DATA: &str =
    "const key='space';export const palette={[key]:'8px',browser:window.document};";
pub(super) const LITERAL_DATA: &str =
    "export const palette={['space']:'8px',browser:window.document};";
pub(super) const ALIAS_CONSUMER: &str = "import {style} from '@vanilla-extract/css';import {palette} from './data';export const box=style({margin:palette.sizes.space});";
pub(super) const ALIAS_DATA: &str = "const first='space';const key=(first as string);export const palette={sizes:{[(key as string)]:'8px',browser:window.document},browser:window.document};";
pub(super) const NUMERIC_CONSUMER: &str = "import {style} from '@vanilla-extract/css';import {palette} from './data';export const box=style({margin:palette[0]});";
pub(super) const NUMERIC_DATA: &str =
    "const key=0;export const palette={[key]:'8px',browser:window.document};";
pub(super) const UNKNOWN_DATA: &str =
    "export const palette={space:'8px',[window.name]:'16px',browser:window.document};";

pub(super) fn assert_static_margin_and_edge(output: &ExtractOutput) {
    assert!(has_static(output, "margin", "8px"), "{:?}", output.styles);
    let margins: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property == "margin" => {
                Some(Some(style.value.as_str()))
            }
            ExtractStyleValue::Dynamic(style) if style.property() == "margin" => Some(None),
            ExtractStyleValue::Static(_)
            | ExtractStyleValue::Dynamic(_)
            | ExtractStyleValue::Typography(_)
            | ExtractStyleValue::Css(_)
            | ExtractStyleValue::Import(_)
            | ExtractStyleValue::FontFace(_)
            | ExtractStyleValue::Keyframes(_) => None,
        })
        .collect();
    assert_eq!(margins, vec![Some("8px")]);
    assert_consumed(output);
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, &output.code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{}", output.code);
    let boxes: Vec<_> = parsed
        .program
        .body
        .iter()
        .filter_map(|statement| {
            let Statement::ExportDeclaration(export) = statement else {
                return None;
            };
            let Declaration::VariableDeclaration(declaration) = &export.declaration else {
                return None;
            };
            Some(declaration.declarations.iter().filter(|declarator| {
                matches!(
                    &declarator.id,
                    BindingPattern::BindingIdentifier(binding) if binding.name == "box"
                )
            }))
        })
        .flatten()
        .collect();
    let [declarator] = boxes.as_slice() else {
        panic!("expected one exported box: {}", output.code)
    };
    assert!(matches!(
        &declarator.init,
        Some(Expression::StringLiteral(_))
    ));
    assert_import(output, "./data");
    assert!(
        output.dependencies.iter().any(|path| path == DATA_PATH),
        "{:?}",
        output.dependencies
    );
}
