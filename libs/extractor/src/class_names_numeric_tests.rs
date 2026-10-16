use crate::{ExtractOption, ExtractOutput, ImportAlias, extract};
use boa_engine::{Context, Source};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;
use std::collections::HashMap;

fn compile(expression: &str) -> Result<ExtractOutput, String> {
    reset_class_map();
    reset_file_map();
    extract(
        "numeric.tsx",
        &format!(
            "import {{ ClassNames }} from '@emotion/react';const App = on => <ClassNames>{{({{css,cx}}) => {expression}}}</ClassNames>;"
        ),
        ExtractOption {
            single_css: true,
            import_main_css: false,
            import_aliases: HashMap::from([(
                "@emotion/react".to_string(),
                ImportAlias::NamedToNamed,
            )]),
            ..ExtractOption::default()
        },
    )
    .map_err(|error| error.to_string())
}

fn execute(code: &str) -> String {
    let arena = Allocator::default();
    let mut parsed = Parser::new(&arena, code, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0, "{code}");
    parsed
        .program
        .body
        .retain(|statement| !matches!(statement, Statement::ImportDeclaration(_)));
    let code = Codegen::new().build(&parsed.program).code;
    let script = format!(
        "{code}\nJSON.stringify([App(true), App(false)].map(classes => classes.split(/\\s+/).filter(Boolean).join(' ')));"
    );
    Context::default()
        .eval(Source::from_bytes(&script))
        .unwrap_or_else(|error| panic!("{script}\n{error}"))
        .as_string()
        .unwrap_or_else(|| panic!("expected serialized class tokens from {script}"))
        .to_std_string_escaped()
}

#[rstest]
#[case("cx(1)", r#"["1","1"]"#)]
#[case("cx(0)", r#"["",""]"#)]
#[case("cx(['caller',[1,0,[2.5]]])", r#"["caller 1 2.5","caller 1 2.5"]"#)]
#[case("cx(1e21,1e-7,0x10)", r#"["1e+21 1e-7 16","1e+21 1e-7 16"]"#)]
#[case("cx(on ? 1 : 0)", r#"["1",""]"#)]
#[case("cx(on ? [0,2] : [3,0])", r#"["2","3"]"#)]
#[case("cx(on && 4)", r#"["4",""]"#)]
#[case("cx(on && 0)", r#"["",""]"#)]
#[case("cx(0 || 5)", r#"["5","5"]"#)]
#[case("cx(6 || 7)", r#"["6","6"]"#)]
#[case("cx(0 ?? 8)", r#"["",""]"#)]
#[case("cx(null ?? 9)", r#"["9","9"]"#)]
#[case("cx(-1)", r#"["-1","-1"]"#)]
#[case("cx(-0)", r#"["",""]"#)]
#[case("cx(+2)", r#"["2","2"]"#)]
#[case("cx(+0)", r#"["",""]"#)]
#[case("cx(['caller',[-1,-0,[+2]]])", r#"["caller -1 2","caller -1 2"]"#)]
#[case("cx(on ? -1 : +2)", r#"["-1","2"]"#)]
#[case("cx(on ? [-0,+2] : [-1,+0])", r#"["2","-1"]"#)]
#[case("cx(on && -1)", r#"["-1",""]"#)]
#[case("cx(on && -0)", r#"["",""]"#)]
#[case("cx(-0 || +2)", r#"["2","2"]"#)]
#[case("cx(-0 ?? +2)", r#"["",""]"#)]
#[case("cx(-1 || +2)", r#"["-1","-1"]"#)]
#[serial]
fn class_names_numeric_classes_execute_when_static(
    #[case] expression: &str,
    #[case] expected: &str,
) {
    // Given: static numeric class leaves, with runtime branch controllers where relevant.
    let output = compile(expression).unwrap_or_else(|error| panic!("{expression}\n{error}"));
    // When: execute the actual emitted render function for both branch states.
    let result = execute(&output.code);
    // Then: nonzero numbers are class text and zero contributes no class.
    assert_eq!(result, expected);
    assert_eq!(output.styles.len(), 0);
}

#[test]
#[serial]
fn class_names_numeric_map_conditions_keep_truthiness() {
    // Given: numbers are conditions, not classes, inside class-map values.
    let output = compile("cx({ kept: 2, omitted: 0, chosen: on })")
        .unwrap_or_else(|error| panic!("{error}"));
    // When: execute both runtime choices.
    let result = execute(&output.code);
    // Then: keys, not the numeric condition values, become classes.
    assert_eq!(result, r#"["kept chosen","kept"]"#);
    assert_eq!(output.styles.len(), 0);
}

#[rstest]
#[case("cx(css({padding:[1,2],opacity:[0,0.5]}),'caller')")]
#[case("cx([css({padding:[1,2],opacity:[0,0.5]}),['caller']])")]
#[case("cx(css({padding:[1,2],opacity:[0,0.5]}),'caller',1,0)")]
#[serial]
fn class_names_numeric_rules_keep_fallbacks(#[case] expression: &str) {
    // Given: numeric CSS fallback arrays alongside ordinary class arguments.
    let output = compile(expression).unwrap_or_else(|error| panic!("{expression}\n{error}"));
    let mut styles: Vec<_> = output
        .styles
        .iter()
        .map(|style| match style {
            crate::extract_style::extract_style_value::ExtractStyleValue::Static(style) => {
                (style.property.as_str(), style.value.as_str(), style.level)
            }
            other => panic!("unexpected style: {other:?}"),
        })
        .collect();
    styles.sort_unstable();
    // When: execute the emitted class expression and inspect the extracted declarations.
    let result = execute(&output.code);
    // Then: numeric rules retain px/unitless fallback semantics at breakpoint zero.
    assert!(result.contains("caller"), "{result}");
    assert_eq!(
        styles,
        [
            ("opacity", "0;opacity:.5", 0),
            ("padding", "1px;padding:2px", 0)
        ]
    );
}

#[rstest]
#[case("cx(getClass(),1)")]
#[case("cx([1,[getClass()]])")]
#[case("cx(on ? 1 : getClass())")]
#[serial]
fn class_names_numeric_arguments_keep_unknown_call_guards(#[case] expression: &str) {
    // Given: a numeric class does not make an unknown call statically readable.
    // When: extract a class expression containing such a call.
    let error = compile(expression)
        .err()
        .unwrap_or_else(|| panic!("expected unknown-call rejection for {expression}"));
    // Then: the existing located build-time rejection is retained.
    assert!(error.starts_with("numeric.tsx:1:"), "{error}");
    assert!(error.contains("getClass()"), "{error}");
    assert!(error.contains("at build time"), "{error}");
}

#[rstest]
#[case("cx(+'2')")]
#[case("cx(-'2')")]
#[case("cx(+true)")]
#[case("cx(-false)")]
#[case("cx(-on)")]
#[case("cx(+on)")]
#[case("cx(-getClass())")]
#[case("cx(+getClass())")]
#[case("cx(1 + 2)")]
#[case("cx(-(1 + 2))")]
#[case("cx(-(-1))")]
#[case("cx([+2,[-getClass()]])")]
#[case("cx(on ? -1 : +getClass())")]
#[serial]
fn class_names_numeric_arguments_reject_nonliteral_operands(#[case] expression: &str) {
    // Given: only exact signed numeric literal leaves are authorized.
    // When: extract a coercion, computed operand, or unknown call.
    let error = compile(expression)
        .err()
        .unwrap_or_else(|| panic!("expected nonliteral rejection for {expression}"));
    // Then: the existing located build-time rejection remains in force.
    assert!(error.starts_with("numeric.tsx:1:"), "{error}");
    assert!(error.contains("at build time"), "{error}");
}
