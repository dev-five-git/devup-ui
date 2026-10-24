use super::*;
use crate::extract_style::{ExtractStyleProperty, style_property::StyleProperty};
use boa_engine::{Context, Source};
use oxc_codegen::Codegen;
use oxc_semantic::SemanticBuilder;
use oxc_transformer::{TransformOptions, Transformer};
use serde_json::Value;

pub(super) const CAPTURED: &str = "const trace=[];let selected=true;const Card=styled.article({...((trace.push('controller'),selected)?{color:(trace.push('leaf'),'red')}:{}),color:'green'});";

pub(super) fn element(filename: &str, props: &str) -> String {
    if filename.ends_with('x') {
        format!("<Card {props} />")
    } else {
        format!("jsx(Card,{{{props}}})")
    }
}

pub(super) fn css_props(filename: &str, css: &str, extra: &str) -> String {
    if filename.ends_with('x') {
        format!("css={{{css}}} {extra}")
    } else {
        format!("css:{css},{extra}")
    }
}

pub(super) fn execute(filename: &str, source: &str, observation: &str) -> Value {
    execute_in_mode(filename, source, observation, true)
}

fn execute_in_mode(filename: &str, source: &str, observation: &str, single_css: bool) -> Value {
    let _state = crate::test_state::TestStateGuard::new();
    reset_class_map();
    reset_file_map();
    let option = ExtractOption {
        single_css,
        import_aliases: std::collections::HashMap::from([(
            "@emotion/react".to_string(),
            crate::ImportAlias::NamedToNamed,
        )]),
        ..ExtractOption::default()
    };
    let bucket = crate::css_bucket(filename, &option);
    let output = extract(
        filename,
        &format!("import {{styled,css}} from '@devup-ui/react';import {{jsx}} from '@emotion/react';{source}"),
        option,
    )
    .unwrap_or_else(|error| panic!("{filename}\n{source}\n{error}"));
    let mut rules = std::collections::BTreeMap::new();
    for style in &output.styles {
        let rule = match style {
            crate::ExtractStyleValue::Static(style) => {
                let StyleProperty::ClassName(name) = style.extract(bucket.as_deref()) else {
                    panic!("static atom must have a class")
                };
                Some((
                    name,
                    serde_json::json!([
                        style.property(),
                        style.value(),
                        style.level(),
                        style
                            .selector()
                            .map(ToString::to_string)
                            .unwrap_or_default(),
                        style.layer()
                    ]),
                ))
            }
            crate::ExtractStyleValue::Dynamic(style) => {
                let StyleProperty::Variable {
                    class_name,
                    variable_name,
                    ..
                } = style.extract(bucket.as_deref())
                else {
                    panic!("dynamic atom must have a variable")
                };
                Some((
                    class_name,
                    serde_json::json!([
                        style.property(),
                        format!("var({variable_name})"),
                        style.level(),
                        style
                            .selector()
                            .map(ToString::to_string)
                            .unwrap_or_default(),
                        style.layer
                    ]),
                ))
            }
            _ => None,
        };
        if let Some((name, rule)) = rule {
            rules.insert(name, rule);
        }
    }
    let arena = Allocator::default();
    let mut program = parse(&arena, filename, &output.code);
    if !filename.ends_with('x') {
        renders(&program);
    }
    let scoping = SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    let options = TransformOptions::default();
    let transformed = Transformer::new(&arena, std::path::Path::new(filename), &options)
        .build_with_scoping(scoping, &mut program);
    assert_eq!(transformed.diagnostics.len(), 0, "{}", output.code);
    let mut aliases = String::new();
    for statement in &program.body {
        if let Statement::ImportDeclaration(import) = statement {
            for specifier in import.specifiers.iter().flatten() {
                if let ImportDeclarationSpecifier::ImportSpecifier(named) = specifier {
                    let imported = named.imported.to_string();
                    let local = named.local.name.as_str();
                    let value = match imported.as_str() {
                        "jsx" | "jsxs" | "jsxDEV" => "jsx",
                        "forwardRef" => "forwardRef",
                        _ => panic!("unexpected runtime import: {imported}"),
                    };
                    if local != value {
                        aliases.push_str("const ");
                        aliases.push_str(local);
                        aliases.push('=');
                        aliases.push_str(value);
                        aliases.push(';');
                    }
                }
            }
        }
    }
    program.body.retain(|statement| {
        !matches!(statement, Statement::ImportDeclaration(_))
            && !matches!(statement, Statement::ExportNamedDeclaration(export)
                if export.specifiers.is_empty())
    });
    let code = Codegen::new().build(&program).code;
    let rules = serde_json::to_string(&rules)
        .unwrap_or_else(|error| panic!("atom map must serialize: {error}"));
    let script = format!(
        r"function forwardRef(render){{return {{render}};}}
function jsx(target,props){{return {{target,props}};}}
function require(name){{if(name==='react')return {{forwardRef}};if(name==='react/jsx-runtime')return {{jsx}};throw new Error(name);}}
function render(element){{return typeof element.target==='string'?element:render(element.target.render(element.props,element.props.ref));}}
const rules={rules};
function atoms(element){{return (element.props.className||'').split(/\s+/).flatMap(name=>rules[name]?[rules[name]]:[]);}}
function values(element,property,level=0,selector=''){{return atoms(element).filter(rule=>rule[0]===property&&rule[2]===level&&rule[3]===selector).map(rule=>{{const value=rule[1];return value.startsWith('var(')?element.props.style[value.slice(4,-1)]:value;}}).sort();}}
function slots(element){{return Object.keys(element.props).filter(key=>key.startsWith('__devupCss'));}}
{aliases}{code}
{observation}"
    );
    let result = Context::default()
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("native CSS-prop execution: {error}\n{code}"));
    let json = result
        .as_string()
        .unwrap_or_else(|| panic!("observation must be JSON"))
        .to_std_string_escaped();
    serde_json::from_str(&json).unwrap_or_else(|error| panic!("{error}\n{json}\n{code}"))
}

#[rstest]
#[case::emotion("emotionCss", false, "1px")]
#[case::native("css", false, "4px")]
#[case::styled("styled.div", true, "4px")]
#[serial]
fn conditional_rule_arrays_keep_one_getter_read_and_consistent_branches(
    #[case] factory: &str,
    #[case] styled: bool,
    #[case] margin: &str,
    #[values(false, true)] single_css: bool,
) {
    let source = format!(
        "import {{css as emotionCss}} from '@emotion/react';const trace=[];const state={{get on(){{trace.push('read');return trace.length===1;}}}};const f=()=>{factory}(state.on?[{{color:'red'}},{{margin:1}}]:[{{color:'blue'}}]);"
    );
    let call = if styled {
        "render({target:f(),props:{}})"
    } else {
        "({target:'div',props:{className:f()}})"
    };
    let observation = format!(
        "const first={call};const firstTrace=trace.slice();const second={call};JSON.stringify([firstTrace,trace,values(first,'color'),values(first,'margin'),values(second,'color'),values(second,'margin')]);"
    );
    let observed = execute_in_mode("test.ts", &source, &observation, single_css);
    assert_eq!(
        observed,
        serde_json::json!([["read"], ["read", "read"], ["red"], [margin], ["blue"], []]),
    );
}

#[path = "controller_evaluation.rs"]
mod controller_evaluation;

#[path = "props_function_evaluation.rs"]
mod props_function_evaluation;

#[path = "props_function_bindings.rs"]
mod props_function_bindings;

#[path = "props_function_order.rs"]
mod props_function_order;

#[path = "props_function_theme.rs"]
mod props_function_theme;

#[path = "implicit_theme_scope.rs"]
mod implicit_theme_scope;

#[path = "local_member_guard.rs"]
mod local_member_guard;

#[path = "integration_semantics.rs"]
mod integration_semantics;

#[path = "integration_execution.rs"]
mod integration_execution;
