use super::*;
use crate::extract_style::style_property::StyleProperty;
use css::class_map::reset_class_map;
use css::file_map::reset_file_map;

mod binding_edges;
mod calls;
mod capture_edges;
mod capture_preservation;
mod captures;
mod core_regressions;
mod create_element;
mod css_prop;
mod errors;
mod expectations;
use expectations::{RequiredError, RequiredValue};
mod evaluation_edges;
mod finite_consumers;
mod finite_css;
mod forwarded;
mod gate_edges;
mod literal_edges;
mod literal_effects;
mod literal_locations;
mod literal_order;
mod literal_routes;
mod literal_selectors;
mod metadata_audit;
mod ordered;
mod repeated;
mod slots;
mod style_order;
mod style_order_w38;
mod styled;
mod type_imports;
mod whole;

const BOX: &str = "import { Box } from '@devup-ui/react';\n";
const EMOTION: &str = "/** @jsxImportSource @emotion/react */\n";
const JSX_RUNTIME: &str = "import { jsx } from 'react/jsx-runtime';\n";

fn compile_with(code: &str, option: ExtractOption) -> Result<ExtractOutput, String> {
    reset_class_map();
    reset_file_map();
    css::debug::set_debug(true);
    let output = extract("a.tsx", code, option).map_err(|error| error.to_string());
    css::debug::set_debug(false);
    output
}

fn compile(code: &str) -> Result<ExtractOutput, String> {
    compile_with(code, ExtractOption::default())
}

fn compile_emotion(code: &str) -> Result<ExtractOutput, String> {
    compile_with(
        code,
        ExtractOption {
            import_aliases: HashMap::from([(
                "@emotion/react".to_string(),
                ImportAlias::NamedToNamed,
            )]),
            ..ExtractOption::default()
        },
    )
}

/// The output of a file that compiles
fn output(source: &str) -> ExtractOutput {
    compile(source).unwrap_or_else(|error| panic!("the file does not compile: {error}"))
}

fn code(source: &str) -> String {
    output(source).code
}

/// The message of a file that does not compile
fn error(source: &str) -> String {
    compile(source)
        .err()
        .unwrap_or_else(|| panic!("the negative fixture compiles: {source}"))
}

/// A style that gives way to a spread: its property, breakpoint, the code
/// setting the variable it reads, the variable and the value it keeps
struct Slot {
    property: String,
    level: u8,
    identifier: String,
    variable: String,
    fallback: String,
}

fn slots(output: &ExtractOutput) -> Vec<Slot> {
    let mut found: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match (style, style.extract(None)) {
            (
                ExtractStyleValue::Dynamic(dynamic),
                Some(StyleProperty::Variable { variable_name, .. }),
            ) => Some(Slot {
                property: dynamic.property().to_string(),
                level: dynamic.level(),
                identifier: captures::materialized(dynamic.identifier(), &output.code),
                variable: variable_name,
                fallback: dynamic.fallback()?.to_string(),
            }),
            _ => None,
        })
        .collect();
    found.sort_by(|a, b| {
        (&a.property, a.level, &a.fallback).cmp(&(&b.property, b.level, &b.fallback))
    });
    found
}

fn static_styles(output: &ExtractOutput) -> Vec<(String, String)> {
    let mut found: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => {
                Some((style.property().to_string(), style.value().to_string()))
            }
            _ => None,
        })
        .collect();
    found.sort();
    found
}

/// What the JavaScript `body` returns, run as the build's output runs it
fn run(body: &str) -> String {
    run_script(&format!("(function() {{ {body} }})()"))
}

/// Run a script directly so whole-file probes do not add a recursive parser frame.
fn run_script(script: &str) -> String {
    let mut context = boa_engine::Context::default();
    let value = context
        .eval(boa_engine::Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("{error}: {script}"));
    value.to_string(&mut context).map_or_else(
        |error| panic!("{error}: {script}"),
        |text| text.to_std_string_escaped(),
    )
}

/// What the code `identifier` of a slot reads once `setup` has bound the
/// props the spreads give: the value as JSON, or `undefined` when it sets none
fn read(identifier: &str, setup: &str) -> String {
    run(&format!(
        "{setup} const slot = {identifier}; return slot === undefined ? 'undefined' : JSON.stringify(slot);"
    ))
}
