use crate::{ExtractOutput, ExtractStyleValue};
use oxc_allocator::{Allocator, TakeIn};
use oxc_ast::ast::Statement;

/// Executes the complete generated module with its declarations and render captures intact.
pub(crate) fn execute(
    location: (&str, Option<&str>),
    output: &ExtractOutput,
    observation: &str,
) -> serde_json::Value {
    let arena = Allocator::default();
    let mut program = super::parse(&arena, location.0, &output.code);
    let mut imports = oxc_allocator::Vec::new_in(&&arena);
    let mut body = oxc_allocator::Vec::new_in(&&arena);
    for statement in program.body {
        match statement {
            Statement::ImportDeclaration(_) => {
                imports.push(statement);
            }
            Statement::ExportDeclaration(mut export) => {
                body.push(Statement::from(export.declaration.take_in(&&arena)));
            }
            Statement::ExportNamedDeclaration(_) => {}
            statement => body.push(statement),
        }
    }
    program.body = imports;
    let imports = oxc_codegen::Codegen::new().build(&program).code;
    program.body = body;
    let code = oxc_codegen::Codegen::new().build(&program).code;
    // The authored module gets its own lexical scope; its `rules` binding must
    // not collide with the canonical harness's atom lookup table.
    let executable = ExtractOutput {
        code: format!("{imports}\n{{\n{code}\n{observation}\n}}"),
        styles: output.styles.clone(),
        map: None,
        css_file: output.css_file.clone(),
        dependencies: output.dependencies.clone(),
        deferred_defaults: output.deferred_defaults.clone(),
    };
    super::css_prop_execution::execute_extracted(location, &executable, "")
}

/// Selects only declarations actually attached by the original renderer.
pub(crate) fn active(
    location: (&str, Option<&str>),
    output: &ExtractOutput,
    props: &str,
) -> Vec<ExtractStyleValue> {
    let names = execute(
        location,
        output,
        &format!(
            "const element=render({{target:Choice,props:{props}}});JSON.stringify(element.props.className||'');"
        ),
    );
    let names = names
        .as_str()
        .unwrap_or_else(|| panic!("renderer className must be a string"));
    output
        .styles
        .iter()
        .filter(|style| match style {
            ExtractStyleValue::LayerOrder(layers) => {
                assert_ne!(
                    layers.layers.len(),
                    0,
                    "layer metadata must carry authored order"
                );
                false
            }
            ExtractStyleValue::Static(_) | ExtractStyleValue::Dynamic(_) => {
                let class = style
                    .extract(location.1)
                    .unwrap_or_else(|| panic!("declaration class identity"));
                let class = match class {
                    crate::extract_style::style_property::StyleProperty::ClassName(name)
                    | crate::extract_style::style_property::StyleProperty::Variable {
                        class_name: name,
                        ..
                    } => name,
                };
                names.split_whitespace().any(|name| name == class)
            }
            other => panic!("unexpected declaration variant: {other:?}"),
        })
        .cloned()
        .collect()
}
