use super::*;
use crate::extract_style::style_property::StyleProperty;
use css::class_map::reset_class_map;
use css::file_map::reset_file_map;

mod binding_edges;
mod boundary_classes;
mod boundary_selection;
mod call_diagnostics;
mod call_spread_attachment;
mod calls;
mod capture_edges;
mod capture_preservation;
mod create_element;
mod create_element_attachment;
mod css_prop;
mod errors;
mod evaluation_edges;
mod forwarded;
mod gate_edges;
mod ordered;
mod pass_through;
mod pass_through_edges;
mod pass_through_known;
mod repeated;
mod slots;
mod spread_repair;
mod style_order;
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
    let script = format!("(function() {{ {body} }})()");
    let mut context = boa_engine::Context::default();
    let value = context
        .eval(boa_engine::Source::from_bytes(script.as_bytes()))
        .unwrap_or_else(|error| panic!("{error}: {script}"));
    value.to_string(&mut context).map_or_else(
        |error| panic!("{error}: {script}"),
        |text| text.to_std_string_escaped(),
    )
}
