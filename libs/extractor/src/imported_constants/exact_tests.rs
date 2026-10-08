use crate::{ExtractOption, ExtractOutput, ExtractStyleValue, ResolvedModule};

pub(super) fn extracted(code: &str, module: &str) -> Result<ExtractOutput, String> {
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let module = module.to_string();
    let resolver = move |specifier: &str, _: &str| {
        (specifier == "./values").then(|| ResolvedModule {
            path: "/src/values.ts".to_string(),
            code: module.clone(),
        })
    };
    crate::extract_with_modules(
        "/src/App.tsx",
        code,
        ExtractOption {
            import_main_css: false,
            ..ExtractOption::default()
        },
        false,
        &resolver,
    )
    .map_err(|error| error.to_string())
}

pub(super) fn static_values(output: &ExtractOutput) -> Vec<String> {
    output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => Some(style.value.clone()),
            _ => None,
        })
        .collect()
}

#[test]
#[serial_test::serial]
fn fn08_computed_enum_when_preceding_members_are_exact() {
    let module = "const STEP = 3; export enum E { A = 2, B = A + STEP, C = E.B + 1, D, Text = 'gr' + 'een' }";
    let output = extracted("import { css } from '@devup-ui/react'; import { E } from './values'; export const s = css({ zIndex: E.B, order: E.C, flexGrow: E.D, color: E.Text });", module).unwrap_or_else(|error| panic!("fn08_computed_enum_when_preceding_members_are_exact: {error}"));
    let values = static_values(&output);
    for expected in ["5", "6", "7", "green"] {
        assert!(values.contains(&expected.to_string()), "{values:?}");
    }
}

#[test]
#[serial_test::serial]
fn fn08_enum_when_a_member_is_uncertain_or_ambient() {
    for module in [
        "export enum E { A = unknown(), B = A + 3, C = 8 }",
        "export declare enum E { A = 2, B = 5 }",
        "export enum E { A = B + 1, B = 5 }",
        "const B = 8; export enum E { A = B + 1, B = 5 }",
        "export enum E { A = 'green', B }",
    ] {
        assert!(extracted("import { css } from '@devup-ui/react'; import { E } from './values'; css({ zIndex: E.B });", module).is_err(), "{module}");
    }
}

#[test]
#[serial_test::serial]
fn jsx12_primitives_when_lexical_bindings_share_names() {
    let output = extracted("import { Box } from '@devup-ui/react'; const color = 'red'; export function f() { const color = 'green'; const n = Math.imul(2, 3); const alias = n + 1; observe(color, alias); return <Box color={color} zIndex={alias} />; } export const g = () => { const color = 'blue'; return <Box color={color} />; }; export const top = <Box color={color} />;", "").unwrap_or_else(|error| panic!("jsx12_primitives_when_lexical_bindings_share_names: {error}"));
    let values = static_values(&output);
    for expected in ["red", "green", "blue", "7"] {
        assert!(values.contains(&expected.to_string()), "{values:?}");
    }
    assert!(!output.code.contains("--"), "{}", output.code);
    assert!(
        output.code.contains("observe(color, alias)"),
        "{}",
        output.code
    );
}

#[test]
#[serial_test::serial]
fn jsx12_primitives_when_nested_scopes_read_imports_and_outer_constants() {
    let output = extracted("import { Box } from '@devup-ui/react'; import { BASE } from './values'; const STEP = 2; export function f() { const n = BASE + STEP; { const n = 9; observe(n); } const value = (`${n}px` as const) satisfies string; return <Box zIndex={n} width={value} />; }", "export const BASE = 5;").unwrap_or_else(|error| panic!("jsx12_primitives_when_nested_scopes_read_imports_and_outer_constants: {error}"));
    assert_eq!(
        static_values(&output),
        vec!["7px".to_string(), "7".to_string()]
    );
    assert_eq!(output.dependencies, vec!["/src/values.ts".to_string()]);
}

#[test]
#[serial_test::serial]
fn jsx12_constants_when_reads_precede_initialization_or_write_the_binding() {
    for body in [
        "const view = <Box color={color} />; var color = 'red'; return view;",
        "const color = 'red'; color = 'blue'; return <Box color={color} />;",
        "const a = compute(); const b = a; return <Box color={b} />;",
    ] {
        let output = extracted(
            &format!("import {{ Box }} from '@devup-ui/react'; export function f() {{ {body} }}"),
            "",
        )
        .unwrap_or_else(|error| {
            panic!(
                "jsx12_constants_when_reads_precede_initialization_or_write_the_binding: {error}"
            )
        });
        assert!(output.code.contains("--"), "{body}: {}", output.code);
    }
}

#[test]
#[serial_test::serial]
fn fn09_math_when_results_or_functions_are_uncertain() {
    for expression in [
        "Math.sin(1)",
        "Math.random()",
        "Math.sqrt(-1)",
        "Math.max()",
        "Math.min()",
        "Math.fround(1e100)",
        "Math.NOPE(1)",
        "Math.imul(unknown(), 2)",
    ] {
        let module = format!("export const VALUE = {expression};");
        assert!(extracted("import { css } from '@devup-ui/react'; import { VALUE } from './values'; css({ zIndex: VALUE });", &module).is_err(), "{expression}");
    }
    for binding in [
        "let Math = source;",
        "function Math() {}",
        "const Math = { imul: unknown };",
        "const undefined = unknown;",
    ] {
        let module = format!("{binding} export const VALUE = Math.imul(undefined, 2);");
        assert!(extracted("import { css } from '@devup-ui/react'; import { VALUE } from './values'; css({ zIndex: VALUE });", &module).is_err(), "{binding}");
    }
}

#[test]
fn fn09_signed_zero_when_exact_math_preserves_js_edges() {
    use crate::build_time_values::exact_math::{Operand, evaluate};
    for (name, arguments, negative) in [
        ("round", vec![-0.5], true),
        ("round", vec![0.5], false),
        ("sign", vec![-0.0], true),
        ("min", vec![0.0, -0.0], true),
        ("max", vec![-0.0, 0.0], false),
        ("fround", vec![-0.0], true),
    ] {
        let arguments: Vec<_> = arguments.into_iter().map(Operand::Number).collect();
        let result =
            evaluate(name, Some(&arguments)).unwrap_or_else(|| panic!("evaluate {name} failed"));
        assert_eq!(result.is_sign_negative(), negative, "{name}");
    }
    for (name, arguments) in [
        ("max", vec![f64::NAN, 1.0]),
        ("min", vec![1.0, f64::NAN]),
        ("sign", vec![f64::NAN]),
        ("round", vec![f64::INFINITY]),
    ] {
        let arguments: Vec<_> = arguments.into_iter().map(Operand::Number).collect();
        assert_eq!(evaluate(name, Some(&arguments)), None, "{name}");
    }
    assert!(super::fold_math("abs", &[super::Constant::Object(std::rc::Rc::default())]).is_none());
}

#[test]
#[serial_test::serial]
fn fn08_exact_prefix_when_later_enum_members_are_uncertain() {
    let output = extracted("import { css } from '@devup-ui/react'; import { E } from './values'; css({ zIndex: E.A });", "const A = 99; const OFFSET = A; export enum E { A = 2, B = A + OFFSET, C = unknown(), D = 8 }").unwrap_or_else(|error| panic!("fn08_exact_prefix_when_later_enum_members_are_uncertain (E.A): {error}"));
    assert_eq!(static_values(&output), vec!["2".to_string()]);
    let output = extracted("import { css } from '@devup-ui/react'; import { E } from './values'; css({ zIndex: E.B });", "const A = 99; const OFFSET = A; export enum E { A = 2, B = A + OFFSET }").unwrap_or_else(|error| panic!("fn08_exact_prefix_when_later_enum_members_are_uncertain (E.B): {error}"));
    assert_eq!(static_values(&output), vec!["101".to_string()]);
}

#[test]
#[serial_test::serial]
fn jsx12_primitive_only_when_no_top_level_value_is_read() {
    let output = extracted("import { Box } from '@devup-ui/react'; export function f() { const color = 'green'; const n = 3 as const; const yes = true; const empty = null; return <Box color={color} zIndex={n} display={yes ? 'block' : 'none'} bg={empty} />; }", "").unwrap_or_else(|error| panic!("jsx12_primitive_only_when_no_top_level_value_is_read: {error}"));
    let values = static_values(&output);
    for expected in ["green", "3", "block"] {
        assert!(values.contains(&expected.to_string()), "{values:?}");
    }
    assert!(!output.code.contains("--"), "{}", output.code);
}

#[test]
#[serial_test::serial]
fn jsx12_values_when_nonfinite_constants_are_not_style_literals() {
    let output = extracted("import { Box } from '@devup-ui/react'; export function f() { const n = Infinity; return <Box zIndex={n} />; }", "").unwrap_or_else(|error| panic!("jsx12_values_when_nonfinite_constants_are_not_style_literals: {error}"));
    assert!(output.code.contains("--"), "{}", output.code);
    assert!(extracted("import { css } from '@devup-ui/react'; import { E } from './values'; css({ zIndex: E.B });", "export enum E { A = Infinity, B }").is_err());
}

#[test]
#[serial_test::serial]
fn jsx12_values_when_parameters_mutation_or_uncertainty_prevent_folding() {
    for body in [
        "const color = Math.random();",
        "let color = 'red'; color = 'blue';",
        "const object = { color: 'red' }; object.color = 'blue'; const color = object.color;",
        "const Math = source; const color = Math.imul(2, 3);",
        "const color = Math.sin(1);",
        "const color = parameter;",
    ] {
        let output = extracted(&format!("import {{ Box }} from '@devup-ui/react'; const color = 'green'; export function f(parameter) {{ {body} return <Box color={{color}} />; }}"), "").unwrap_or_else(|error| panic!("jsx12_values_when_parameters_mutation_or_uncertainty_prevent_folding: {error}"));
        assert!(output.code.contains("--"), "{body}: {}", output.code);
    }
    let output = extracted("import { Box } from '@devup-ui/react'; const color = 'red'; export function f(color) { return <Box color={color} />; }", "").unwrap_or_else(|error| panic!("jsx12_values_when_parameters_mutation_or_uncertainty_prevent_folding (param): {error}"));
    assert!(output.code.contains("--"), "{}", output.code);
}
