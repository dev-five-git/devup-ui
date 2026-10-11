use super::exact_tests::{extracted, static_values};

#[test]
#[serial_test::serial]
fn fn09_math_when_local_and_imported_values_are_exact() {
    for (expression, expected) in [
        ("Math.abs(-2)", "2"),
        ("Math.ceil(1.2)", "2"),
        ("Math.floor(1.8)", "1"),
        ("Math.round(-2.5)", "-2"),
        ("Math.trunc(-1.8)", "-1"),
        ("Math.sign(-2)", "-1"),
        ("Math.max(1, 3, 2)", "3"),
        ("Math.min(1, -3, 2)", "-3"),
        ("Math.sqrt(4)", "2"),
        ("Math.fround(16777217)", "16777216"),
        ("Math.imul(4294967295, 5)", "-5"),
        ("Math.clz32(0)", "32"),
        ("Math.PI", "3.141592653589793"),
        ("Math.E", "2.718281828459045"),
        ("Math.LN2", "0.6931471805599453"),
        ("Math.LN10", "2.302585092994046"),
        ("Math.LOG2E", "1.4426950408889634"),
        ("Math.LOG10E", "0.4342944819032518"),
        ("Math.SQRT2", "1.4142135623730951"),
        ("Math.SQRT1_2", "0.7071067811865476"),
        ("Math.imul(2147483647, 2)", "-2"),
        ("Math.imul(3.9, 2.9)", "6"),
        ("Math.clz32(-1)", "0"),
        ("Math.clz32(4294967297)", "31"),
        ("Math.fround(0.1)", "0.10000000149011612"),
        ("Math.round(-0.5)", "0"),
        ("Math.sign(-0)", "0"),
        ("Math.max(-0, 0)", "0"),
        ("Math.min(0, -0)", "0"),
        ("Math.imul('3.9', true)", "3"),
        ("Math.clz32(null)", "32"),
        ("Math.clz32(undefined)", "32"),
        ("Math.clz32(NaN)", "32"),
        ("Math.clz32(Infinity)", "32"),
        ("Math.max('1', 2)", "2"),
    ] {
        for imported in [false, true] {
            let module = format!("export const VALUE = {expression};");
            let binding = if imported {
                "import { VALUE } from './values';".to_string()
            } else {
                format!("function compute() {{ return {expression}; }} const VALUE = compute();")
            };
            let output = extracted(&format!("import {{ css }} from '@devup-ui/react'; {binding} export const s = css({{ zIndex: VALUE }});"), &module).unwrap_or_else(|error| panic!("{expression}, imported={imported}: {error}"));
            assert_eq!(
                static_values(&output)
                    .iter()
                    .map(|value| value
                        .parse::<f64>()
                        .unwrap_or_else(|error| panic!("parse f64: {error}"))
                        .to_bits())
                    .collect::<Vec<_>>(),
                vec![
                    expected
                        .parse::<f64>()
                        .unwrap_or_else(|error| panic!("parse expected f64: {error}"))
                        .to_bits()
                ],
                "{expression}, imported={imported}"
            );
        }
    }
}
