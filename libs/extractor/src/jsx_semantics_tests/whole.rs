//! Runs the whole code a file compiles to, as a bundler runs it, with a fake
//! React that records the props each element is built with.

use super::*;
use oxc_allocator::Allocator;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::SourceType;
use oxc_transformer::{JsxRuntime, TransformOptions, Transformer};
use std::path::Path;

/// `code` with its types stripped and its JSX written as calls of `h`
fn plain_script(code: &str) -> String {
    let allocator = Allocator::default();
    let mut program = Parser::new(&allocator, code, SourceType::tsx())
        .parse()
        .program;
    let scoping = SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    let mut options = TransformOptions::default();
    options.jsx.runtime = JsxRuntime::Classic;
    options.jsx.pragma = Some("h".to_string());
    let _ = Transformer::new(&allocator, Path::new("a.tsx"), &options)
        .build_with_scoping(scoping, &mut program);
    Codegen::new()
        .build(&program)
        .code
        .lines()
        .filter(|line| !line.starts_with("import "))
        .map(|line| line.strip_prefix("export ").unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// What `probe` gives once the code `source` compiles to has run, as JSON,
/// and the `trace` that code wrote while it ran
pub(super) struct Evaluation {
    pub element: serde_json::Value,
    pub trace: serde_json::Value,
}

/// Compile `source`, run what it compiles to, then evaluate `probe` with
/// `elements` building plain objects: `{ type, props, children }`
pub(super) fn evaluate(source: &str, probe: &str) -> Evaluation {
    let compiled = output(source).code;
    let script = plain_script(&compiled);
    let joined = run(&format!(
        "const trace = []; const h = (type, props, ...children) => ({{ type, props: props && {{ ...props }}, children }}); const jsx = (type, props) => ({{ type, props: props && {{ ...props }} }}); const createElement = h; const React = {{ createElement: h }}; const ReactNS = React; {script}\nreturn JSON.stringify([{probe}, trace]);"
    ));
    let (element, trace): (serde_json::Value, serde_json::Value) =
        serde_json::from_str(&joined).unwrap_or_else(|error| panic!("{error}: {joined}"));
    Evaluation { element, trace }
}
