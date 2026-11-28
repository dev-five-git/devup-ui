//! A value read at runtime takes the unit a literal of its property takes,
//! where the element sets it, so `p={m}` with `m = 2` pads by `8px` as
//! `p={2}` does.

use super::*;
use rstest::rstest;
use serial_test::serial;

/// The values the element `a(m)` builds sets on its `style`
fn values(prop: &str, input: &str) -> serde_json::Value {
    let source = format!("{BOX}export const a=(m,on=true)=><Box {prop}/>;");
    whole::evaluate(
        &source,
        &format!("Object.values(a({input}).props.style ?? {{}})"),
    )
    .element
}

#[rstest]
#[case("p={m}", "2", serde_json::json!(["8px"]))]
#[case("p={m}", "'2'", serde_json::json!(["8px"]))]
#[case("p={m}", "-1.5", serde_json::json!(["-6px"]))]
#[case("p={m}", "'2rem'", serde_json::json!(["2rem"]))]
#[case("p={m}", "'auto'", serde_json::json!(["auto"]))]
#[case("w={[m, null, m]}", "1", serde_json::json!(["4px", "4px"]))]
#[case("_hover={{ mx: m }}", "3", serde_json::json!(["12px", "12px"]))]
#[case("p={on ? m : 1}", "5", serde_json::json!(["20px"]))]
#[case("fontSize={m}", "4", serde_json::json!(["16px"]))]
#[case("transitionDuration={m}", "3", serde_json::json!(["3ms"]))]
#[case("opacity={m}", "0.5", serde_json::json!([0.5]))]
#[case("zIndex={m}", "2", serde_json::json!([2]))]
#[case("lineHeight={m}", "2", serde_json::json!([2]))]
#[case("color={m}", "2", serde_json::json!([2]))]
#[serial]
fn runtime_values_take_the_units_a_literal_takes(
    #[case] prop: &str,
    #[case] input: &str,
    #[case] expected: serde_json::Value,
) {
    assert_eq!(values(prop, input), expected);
}

/// `null`, `undefined` and `false` still leave the declaration out: React
/// sets no custom property they hold
#[rstest]
#[case("null", serde_json::json!([null]))]
#[case("undefined", serde_json::json!([null]))]
#[case("false", serde_json::json!([false]))]
#[serial]
fn absent_runtime_values_stay_absent(#[case] input: &str, #[case] expected: serde_json::Value) {
    assert_eq!(values("p={m}", input), expected);
}

#[rstest]
#[case("p={2}")]
#[case("p=\"2\"")]
#[case("opacity={m}")]
#[case("color={m}")]
#[case("p={`${m}px`}")]
#[serial]
fn no_helper_where_no_runtime_number_is_converted(#[case] prop: &str) {
    let code = code(&format!("{BOX}export const a=(m)=><Box {prop}/>;"));
    assert!(!code.contains("__devupScale"), "{code}");
}

#[test]
#[serial]
fn one_helper_serves_every_value_and_avoids_taken_names() {
    let code = code(&format!(
        "{BOX}const __devupScale=1;export const a=(m,n)=><Box p={{m}} m={{n}}/>;"
    ));
    assert_eq!(
        code.matches("import { __devupScale as __devupScale_ } from \"@devup-ui/react/scale\";")
            .count(),
        1,
        "{code}"
    );
    assert_eq!(code.matches("__devupScale_(").count(), 2, "{code}");
}

/// The conversion is imported from the configured package, so a package
/// re-exporting Devup UI serves it too
#[test]
#[serial]
fn the_import_follows_the_configured_package() {
    let output = compile_with(
        "import { Box } from '@my/ui';
export const a=(m)=><Box p={m}/>;",
        ExtractOption {
            package: "@my/ui".into(),
            ..ExtractOption::default()
        },
    )
    .unwrap_or_else(|error| panic!("{error}"));
    assert!(
        output
            .code
            .contains("import { __devupScale } from \"@my/ui/scale\";"),
        "{}",
        output.code
    );
}

/// A `CommonJS` file requires the conversion, as it requires `forwardRef`
#[test]
#[serial]
fn commonjs_files_require_the_conversion() {
    let output = extract(
        "a.cjs",
        "const { jsx } = require('react/jsx-runtime');\nconst { Box } = require('@devup-ui/react');\nexports.a=(m)=>jsx(Box,{p:m});",
        ExtractOption::default(),
    );
    let output = output.unwrap_or_else(|error| panic!("{error}"));
    assert!(
        output
            .code
            .contains("const __devupScale = require(\"@devup-ui/react/scale\").__devupScale;"),
        "{}",
        output.code
    );
}

/// Compiling the output again leaves the import and its calls as they are
#[test]
#[serial]
fn compiled_output_compiles_to_itself() {
    let first = code(&format!("{BOX}export const a=(m)=><Box p={{m}}/>;"));
    let second = code(&first);
    assert_eq!(
        second
            .matches("import { __devupScale } from \"@devup-ui/react/scale\";")
            .count(),
        1,
        "{second}"
    );
    assert!(second.contains("__devupScale(m)"), "{second}");
}

/// Emotion's numbers are pixels, so a file using it keeps its own conversion
#[test]
#[serial]
fn emotion_files_keep_their_own_units() {
    let output = compile_emotion(&format!(
        "{EMOTION}{BOX}export const a=(m)=><Box p={{m}} css={{{{color:'red'}}}}/>;"
    ))
    .unwrap_or_else(|error| panic!("{error}"));
    assert!(!output.code.contains("__devupScale"), "{}", output.code);
}

#[rstest]
#[case("export const A=styled.div`padding: ${p=>p.m};`;")]
#[case("export const A=styled.div(p=>({padding:p.m}));")]
#[case("export const A=styled.div(p=>({p:p.m}));")]
#[serial]
fn styled_values_read_at_runtime_take_devup_units(#[case] definition: &str) {
    let output = output(&format!(
        "import {{ styled }} from '@devup-ui/react';\n{definition}"
    ));
    assert!(
        output
            .code
            .contains("import { __devupScale } from \"@devup-ui/react/scale\";"),
        "{}",
        output.code
    );
    assert_eq!(
        crate::styled_source_tests::captured_render::execute(
            ("a.tsx", None),
            &output,
            concat!(
                "const element=render({target:A,props:{",
                "m:2}});JSON.stringify(Object.values(element.props.style));"
            )
        ),
        serde_json::json!(["8px"]),
        "{}",
        output.code
    );
}
