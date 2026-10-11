use rstest::rstest;
use serial_test::serial;

use super::consumer_scheduling::PRODUCER;
use super::demand_support::{TestResult, reset, run};
use super::mixed_support::has_static;

#[rstest]
#[case(true, "blue")]
#[case(false, "red")]
#[serial]
fn consumer_helper_when_if_selects_one_branch_observes_only_its_value(
    #[case] enabled: bool,
    #[case] color: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        concat!(
            "import {{css}} from '@devup-ui/react';import {{token,read,other,count}} from './producer';",
            "const enabled={enabled};if(enabled){{css({{color:read()}})}}else{{css({{color:other()}})}}",
            "export const check=css({{margin:token,padding:count()+'px'}});"
        ),
        enabled = enabled
    );
    // When
    let output = run(
        "/consumer-if.ts",
        &source,
        &[("./producer", "/counter-owner.ts", PRODUCER)],
    )?;
    // Then
    assert!(has_static(&output, "color", color), "{:?}", output.styles);
    assert!(has_static(&output, "padding", "1px"), "{:?}", output.styles);
    Ok(())
}

#[rstest]
#[case(
    "true ? <Box color={read()}/> : <Box color={other()}/>",
    "1px",
    Some("blue")
)]
#[case(
    "false ? <Box color={read()}/> : <Box color={other()}/>",
    "1px",
    Some("red")
)]
#[case("true && <Box color={read()}/>", "1px", Some("blue"))]
#[case("false && <Box color={read()}/>", "0", None)]
#[case("true || <Box color={read()}/>", "0", None)]
#[case("false || <Box color={read()}/>", "1px", Some("blue"))]
#[case("null ?? <Box color={read()}/>", "1px", Some("blue"))]
#[case("undefined ?? <Box color={read()}/>", "1px", Some("blue"))]
#[case("false ?? <Box color={read()}/>", "0", None)]
#[case("0 ?? <Box color={read()}/>", "0", None)]
#[case("'present' ?? <Box color={read()}/>", "0", None)]
#[serial]
fn consumer_helper_when_jsx_guard_selects_or_skips_it_has_exact_calls(
    #[case] view: &str,
    #[case] calls: &str,
    #[case] color: Option<&str>,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{Box,css}} from '@devup-ui/react';import {{token,read,other,count}} from './producer';export const view={view};export const check=css({{margin:token,padding:count()+'px'}});"
    );
    // When
    let output = run(
        "/consumer-jsx-guards.tsx",
        &source,
        &[("./producer", "/counter-owner.ts", PRODUCER)],
    )?;
    // Then
    assert!(has_static(&output, "padding", calls), "{:?}", output.styles);
    if let Some(color) = color {
        assert!(has_static(&output, "color", color), "{:?}", output.styles);
    }
    Ok(())
}

#[rstest]
#[case(false, false, "0")]
#[case(false, true, "0")]
#[case(true, true, "0")]
#[case(true, false, "1px")]
#[serial]
fn consumer_helper_when_nested_guards_select_it_runs_once_or_not_at_all(
    #[case] outer: bool,
    #[case] inner: bool,
    #[case] calls: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{Box,css}} from '@devup-ui/react';import {{token,read,count}} from './producer';const outer={outer},inner={inner};if(outer){{inner || <Box color={{read()}}/>;}}export const check=css({{margin:token,padding:count()+'px'}});"
    );
    // When
    let output = run(
        "/consumer-nested.tsx",
        &source,
        &[("./producer", "/counter-owner.ts", PRODUCER)],
    )?;
    // Then
    assert!(has_static(&output, "padding", calls), "{:?}", output.styles);
    if outer && !inner {
        assert!(has_static(&output, "color", "blue"), "{:?}", output.styles);
    }
    Ok(())
}

#[test]
#[serial]
fn consumer_guard_when_outer_branch_is_false_skips_throwing_inner_read() -> TestResult {
    // Given
    reset();
    let source = concat!(
        "import {Box,css} from '@devup-ui/react';import {token,read,count} from './producer';",
        "const outer=false,absent=null;if(outer){absent.member || <Box color={read()}/>;}",
        "export const check=css({margin:token,padding:count()+'px'});"
    );
    // When
    let output = run(
        "/consumer-guard-order.tsx",
        source,
        &[("./producer", "/counter-owner.ts", PRODUCER)],
    )?;
    // Then
    assert!(has_static(&output, "padding", "0"), "{:?}", output.styles);
    Ok(())
}
