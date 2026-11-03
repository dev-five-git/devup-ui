use super::*;
use oxc_allocator::{Allocator, CloneIn, TakeIn};
use oxc_ast::ast::{Declaration, Expression, JSXAttributeValue, Statement};
use oxc_ast_visit::{VisitMut, walk_mut};
use oxc_span::SourceType;

fn declared_names(declaration: &Declaration<'_>) -> Vec<String> {
    match declaration {
        Declaration::VariableDeclaration(variable) => variable
            .declarations
            .iter()
            .flat_map(|declarator| declarator.id.get_binding_identifiers())
            .map(|binding| binding.name.to_string())
            .collect(),
        declaration => declaration
            .id()
            .iter()
            .map(|id| id.name.to_string())
            .collect(),
    }
}

#[rstest]
#[case("export const color = 'red';", "color", "color")]
#[case(
    "export function getColor() { return 'red'; }",
    "getColor()",
    "getColor"
)]
#[case("export let color = 'red';", "color", "color")]
#[case("export var color = 'red';", "color", "color")]
#[case("export const color = 'red', other = 'blue';", "color", "color,other")]
#[case("const color = 'red'; export {color};", "color", "color")]
#[case(
    "const color = 'red'; export {color as publicColor};",
    "color",
    "publicColor"
)]
#[case(
    "export default function getColor() { return 'red'; }",
    "getColor()",
    "default"
)]
#[case("export const color = 'red';", "p.color", "color")]
#[serial]
fn public_names_survive_when_callback_captures_exports(
    #[case] declaration: &str,
    #[case] value: &str,
    #[case] names: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{styled}} from '@devup-ui/react'; {declaration} const rules = p => ({{color:{value}}}); export const Choice = styled.div(rules);"
    );
    // When
    let output = extract("export-capture.tsx", &source, ExtractOption::default())?;
    // Then
    let allocator = Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, &output.code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let mut exports = Vec::new();
    for statement in &parsed.program.body {
        match statement {
            Statement::ExportDeclaration(export) => {
                exports.extend(declared_names(&export.declaration));
            }
            Statement::ExportNamedDeclaration(export) => exports.extend(
                export
                    .specifiers
                    .iter()
                    .map(|specifier| specifier.exported.name().to_string()),
            ),
            Statement::ExportDefaultDeclaration(_) => exports.push("default".into()),
            _ => {}
        }
    }
    exports.sort();
    let mut expected: Vec<_> = names.split(',').chain(["Choice"]).collect();
    expected.sort_unstable();
    assert_eq!(exports, expected, "{}", output.code);
    Ok(())
}

/// Observe the generated element's style values using the same JSX-to-Boa
/// adapter as the capture hygiene tests, with getters for the parsed exports.
fn consumer_values(code: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    struct RenderStyle<'a>(&'a Allocator);
    impl<'a> VisitMut<'a> for RenderStyle<'a> {
        fn visit_expression(&mut self, expression: &mut Expression<'a>) {
            if let Expression::JSXElement(element) = expression {
                let value = element
                    .opening_element
                    .attributes
                    .iter()
                    .find_map(|attribute| {
                        let attribute = attribute.as_attribute()?;
                        attribute
                            .name
                            .as_identifier()
                            .filter(|name| name.name == "style")?;
                        match attribute.value.as_ref()? {
                            JSXAttributeValue::ExpressionContainer(container) => {
                                container.expression.as_expression()
                            }
                            JSXAttributeValue::StringLiteral(_)
                            | JSXAttributeValue::Element(_)
                            | JSXAttributeValue::Fragment(_) => None,
                        }
                    });
                assert!(value.is_some(), "expected generated style object");
                if let Some(value) = value {
                    *expression = value.clone_in(self.0);
                }
                return;
            }
            walk_mut::walk_expression(self, expression);
        }
    }
    let allocator = Allocator::default();
    let mut parsed = oxc_parser::Parser::new(&allocator, code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    RenderStyle(&allocator).visit_program(&mut parsed.program);
    let mut getters = Vec::new();
    let mut body = oxc_allocator::Vec::new_in(&&allocator);
    for statement in parsed.program.body {
        match statement {
            Statement::ImportDeclaration(_) => {}
            Statement::ExportDeclaration(mut export) => {
                getters.extend(
                    declared_names(&export.declaration)
                        .iter()
                        .map(|name| format!("get {name}() {{ return {name}; }}")),
                );
                body.push(Statement::from(export.declaration.take_in(&&allocator)));
            }
            Statement::ExportNamedDeclaration(export) => {
                getters.extend(export.specifiers.iter().map(|specifier| {
                    format!(
                        "get {}() {{ return {}; }}",
                        specifier.exported.name(),
                        specifier.local.name()
                    )
                }));
            }
            statement => body.push(statement),
        }
    }
    parsed.program.body = body;
    let compiled = oxc_codegen::Codegen::new().build(&parsed.program).code;
    let script = format!(
        "const __devupForwardRef = fn => props => fn(props, undefined); {compiled}\nconst consumer = {{{}}}; const before = Object.values(consumer.Choice({{}})); consumer.update(); JSON.stringify([consumer.color, before, Object.values(consumer.Choice({{}}))]);",
        getters.join(",")
    );
    let json = boa_engine::Context::default()
        .eval(boa_engine::Source::from_bytes(&script))
        .map_err(|error| format!("consumer evaluation failed: {error}\n{script}"))?
        .as_string()
        .ok_or("expected JSON")?
        .to_std_string_escaped();
    Ok(serde_json::from_str(&json)?)
}

#[rstest]
#[case("<div />")]
#[case("<div style />")]
#[case("<div style='red' />")]
#[case("<div style={/* missing expression */} />")]
#[case("<div {...props} />")]
fn consumer_oracle_when_generated_style_is_missing_rejects(#[case] jsx: &str) {
    // Given
    let code = format!("const Choice = () => {jsx};");
    // When
    let result = std::panic::catch_unwind(|| consumer_values(&code));
    // Then
    assert!(result.is_err(), "missing style must fail before evaluation");
}

#[rstest]
#[case("export let color = 'red';", "color")]
#[case("let color = 'red'; export {color};", "color")]
#[case(
    "export let color = 'red'; function getColor() { return color; }",
    "getColor()"
)]
#[serial]
fn consumer_and_rendered_values_stay_live_when_user_scopes_shadow_capture(
    #[case] declaration: &str,
    #[case] value: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{styled}} from '@devup-ui/react'; {declaration} export function update() {{ color = 'blue'; }} const rules = p => ({{color:{value}}}); function make(color) {{ return styled.div(rules); }} export const Choice = make('green');"
    );
    // When
    let output = extract("export-live.tsx", &source, ExtractOption::default())?;
    // Then
    assert_eq!(
        consumer_values(&output.code)?,
        serde_json::json!(["blue", ["red"], ["blue"]])
    );
    Ok(())
}
