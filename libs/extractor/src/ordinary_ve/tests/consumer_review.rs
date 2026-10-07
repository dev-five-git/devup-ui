use rstest::rstest;
use serial_test::serial;

use super::consumer_support::located_failure;
use super::demand_support::{TestResult, reset, run};
use super::mixed_support::has_static;

#[rstest]
#[case("external(COLORS);", "COLORS);")]
#[case("const alias=COLORS;external(alias);", "alias);")]
#[serial]
fn consumer_seed_rejects_stale_data_when_excluded_runtime_code_receives_its_object(
    #[case] handoff: &str,
    #[case] site: &str,
) -> TestResult {
    // Given
    reset();
    let producer = format!(
        "import {{createTheme}} from '@vanilla-extract/css';export const [theme,vars]=createTheme({{space:'8px'}});export const COLORS={{fg:'blue'}};\n{handoff}"
    );
    let place = crate::locate(
        "/mutable-producer.ts",
        &producer,
        producer.find(site).ok_or("handoff site missing")?,
    );
    let source = "import {css} from '@devup-ui/react';import {vars,COLORS} from './producer';export const box=css({margin:vars.space,color:COLORS.fg});";
    // When
    let result = run(
        "/mutable-consumer.ts",
        source,
        &[("./producer", "/mutable-producer.ts", &producer)],
    );
    // Then
    located_failure(result, &place, "may be changed");
    Ok(())
}

#[test]
#[serial]
fn consumer_seed_keeps_exact_data_when_excluded_code_receives_only_a_primitive() -> TestResult {
    // Given
    reset();
    let producer = "import {createTheme} from '@vanilla-extract/css';export const [theme,vars]=createTheme({space:'8px'});export const COLORS={fg:'blue'};external(COLORS.fg);";
    let source = "import {css} from '@devup-ui/react';import {vars,COLORS} from './producer';export const box=css({margin:vars.space,color:COLORS.fg});";
    // When
    let output = run(
        "/primitive-consumer.ts",
        source,
        &[("./producer", "/primitive-producer.ts", producer)],
    )?;
    // Then
    assert!(has_static(&output, "color", "blue"));
    assert!(has_static(&output, "margin", "var(--space-0-1)"));
    let allocator = oxc_allocator::Allocator::default();
    let parsed =
        oxc_parser::Parser::new(&allocator, &output.code, oxc_span::SourceType::ts()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let semantic = oxc_semantic::SemanticBuilder::new()
        .build(&parsed.program)
        .semantic;
    for name in ["vars", "COLORS"] {
        let references = semantic
            .scoping()
            .get_root_binding(name.into())
            .map_or(0, |symbol| {
                semantic.scoping().get_resolved_reference_ids(symbol).len()
            });
        assert_eq!(
            references, 0,
            "compiled styling still reads {name}: {}",
            output.code
        );
    }
    Ok(())
}

#[rstest]
#[case("module.exports={space:'8px'};")]
#[case("exports.space='8px';")]
#[serial]
fn native_data_control_accepts_a_static_commonjs_helper(#[case] helper: &str) -> TestResult {
    // Given
    reset();
    let source = concat!(
        "import {style} from '@vanilla-extract/css';import {space} from './tokens';export const box=style({",
        "margin:space});"
    );
    // When
    let output = run(
        "/native-cjs.ts",
        source,
        &[("./tokens", "/tokens.js", helper)],
    )?;
    // Then
    assert!(has_static(&output, "margin", "8px"));
    Ok(())
}

#[rstest]
#[case("module.exports={space:'8px'};")]
#[case("exports.space='8px';")]
#[serial]
fn consumer_seed_accepts_a_static_commonjs_helper_when_native_data_uses_it(
    #[case] helper: &str,
) -> TestResult {
    // Given
    reset();
    let source = "import {css} from '@devup-ui/react';import {vars} from './producer';export const box=css({margin:vars.space});";
    let producer = "import {createTheme} from '@vanilla-extract/css';import {space} from './tokens';export const [theme,vars]=createTheme({space});const browser=window.document;";
    // When
    let output = run(
        "/consumer-cjs.ts",
        source,
        &[
            ("./producer", "/cjs-producer.ts", producer),
            ("./tokens", "/tokens.js", helper),
        ],
    )?;
    // Then
    assert!(has_static(&output, "margin", "var(--space-0-1)"));
    assert!(!output.code.contains("vars.space"), "{}", output.code);
    Ok(())
}

#[test]
#[serial]
fn consumer_required_read_errors_when_a_native_producer_has_no_requested_export() -> TestResult {
    // Given
    reset();
    let source = concat!(
        "import {css} from '@devup-ui/react';import {missing} from './producer';export const box=css({",
        "color:missing});"
    );
    let producer = "import {createTheme} from '@vanilla-extract/css';export const [theme,vars]=createTheme({space:'8px'});";
    let place = crate::locate(
        "/missing-consumer.ts",
        source,
        source.rfind("missing").ok_or("missing read absent")?,
    );
    // When
    let result = run(
        "/missing-consumer.ts",
        source,
        &[("./producer", "/missing-producer.ts", producer)],
    );
    // Then
    located_failure(result, &place, "missing");
    Ok(())
}

#[test]
#[serial]
fn consumer_required_read_preserves_undefined_when_the_export_really_exists() -> TestResult {
    // Given
    reset();
    let source = "import {css} from '@devup-ui/react';import {optional} from './producer';export const box=css({color:optional??'blue'});";
    let producer = "import {createTheme} from '@vanilla-extract/css';export const [theme,vars]=createTheme({space:'8px'});export const optional=undefined;";
    // When
    let output = run(
        "/undefined-consumer.ts",
        source,
        &[("./producer", "/undefined-producer.ts", producer)],
    )?;
    // Then
    assert!(has_static(&output, "color", "blue"));
    Ok(())
}

#[rstest]
#[case(("const flag=window.name;", "m", "flag?vars.space:'2px'", "flag"))]
#[case(("const suffix=window.name;", "w", "vars.space+suffix", "suffix"))]
#[serial]
fn consumer_leaf_stays_static_when_a_module_binding_supplies_a_runtime_box_value(
    #[case] fixture: (&str, &str, &str, &str),
) -> TestResult {
    // Given
    reset();
    let (declaration, property, expression, runtime) = fixture;
    let source = format!(
        "import {{Box}} from '@devup-ui/react';import {{vars}} from './producer';{declaration}export const view=<Box {property}={{{expression}}}/>;"
    );
    let producer = "import {createTheme} from '@vanilla-extract/css';export const [theme,vars]=createTheme({space:'8px'});";
    // When
    let output = run(
        "/runtime-leaf.tsx",
        &source,
        &[("./producer", "/runtime-producer.ts", producer)],
    )?;
    // Then: only the native leaf is a build-time value; the page still owns the other input.
    assert!(output.code.contains("<div"), "{}", output.code);
    assert!(output.code.contains("window.name"), "{}", output.code);
    assert!(output.code.contains(runtime), "{}", output.code);
    assert!(!output.code.contains("vars.space"), "{}", output.code);
    assert!(
        output.code.contains("var(--space-0-1)")
            || has_static(&output, "margin", "var(--space-0-1)"),
        "{} {:?}",
        output.code,
        output.styles
    );
    Ok(())
}
