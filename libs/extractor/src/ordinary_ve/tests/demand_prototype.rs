use rstest::rstest;
use serial_test::serial;

use super::demand_support::{TestResult, assert_import, reset, run};
use super::mixed_support::{assert_consumed, has_static};

const PROTOTYPE: &str = "export const tokens={__proto__:{space:'8px'},browser:window.document};throw new Error('unrelated prototype sibling');";
const CONSUMER: &str = "import {style} from '@vanilla-extract/css';import {tokens} from './prototype';export const box=style({margin:tokens.space});";

#[rstest]
#[case("ts")]
#[case("tsx")]
#[case("js")]
#[case("jsx")]
#[case("mjs")]
#[case("css.ts")]
#[case("css.js")]
#[serial]
fn demand_inherited_property_emits_css_when_only_space_is_required(
    #[case] suffix: &str,
) -> TestResult {
    // Given
    reset();
    // When
    let output = run(
        &format!("/prototype-consumer.{suffix}"),
        CONSUMER,
        &[("./prototype", "/prototype.ts", PROTOTYPE)],
    )?;
    // Then
    assert!(has_static(&output, "margin", "8px"), "{:?}", output.styles);
    assert_consumed(&output);
    assert_import(&output, "./prototype");
    assert!(
        output
            .dependencies
            .iter()
            .any(|path| path == "/prototype.ts")
    );
    Ok(())
}

#[rstest]
#[case(
    "export const tokens={__proto__:{__proto__:{space:'8px'},browser:window.document},browser:window.document};",
    "tokens.space"
)]
#[case(
    "export const tokens={__proto__:{spacing:{__proto__:{small:'8px'},browser:window.document}},browser:window.document};",
    "tokens.spacing.small"
)]
#[case(
    "export const tokens={spacing:{__proto__:{small:'8px'},browser:window.document},browser:window.document};",
    "tokens.spacing.small"
)]
#[serial]
fn demand_nested_prototype_emits_css_when_the_required_path_is_inherited(
    #[case] producer: &str,
    #[case] member: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';import {{tokens}} from './prototype';export const box=style({{margin:{member}}});"
    );
    // When
    let output = run(
        "/nested.ts",
        &source,
        &[("./prototype", "/prototype.ts", producer)],
    )?;
    // Then
    assert!(has_static(&output, "margin", "8px"), "{:?}", output.styles);
    Ok(())
}

#[rstest]
#[case("export const tokens={__proto__:{space:'8px'},space:'12px',browser:window.document};")]
#[case("export const tokens={space:'12px',__proto__:{space:'8px'},browser:window.document};")]
#[case(
    "export const tokens={__proto__:{space:window.document},space:'12px',browser:window.document};"
)]
#[serial]
fn demand_own_property_wins_when_it_shadows_an_inherited_value(
    #[case] producer: &str,
) -> TestResult {
    // Given
    reset();
    // When
    let output = run(
        "/shadow.ts",
        CONSUMER,
        &[("./prototype", "/prototype.ts", producer)],
    )?;
    // Then
    assert!(has_static(&output, "margin", "12px"), "{:?}", output.styles);
    assert!(!has_static(&output, "margin", "8px"));
    Ok(())
}

#[rstest]
#[case("tokens.space", "8px", "24px")]
#[case("tokens['__proto__'].space", "24px", "8px")]
#[serial]
fn demand_computed_proto_key_is_own_data_when_a_literal_prototype_also_exists(
    #[case] member: &str,
    #[case] expected: &str,
    #[case] other: &str,
) -> TestResult {
    // Given
    reset();
    let producer = "export const tokens={__proto__:{space:'8px'},['__proto__']:{space:'24px'},browser:window.document};";
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';import {{tokens}} from './prototype';export const box=style({{margin:{member}}});"
    );
    // When
    let output = run(
        "/computed.ts",
        &source,
        &[("./prototype", "/prototype.ts", producer)],
    )?;
    // Then
    assert!(
        has_static(&output, "margin", expected),
        "{:?}",
        output.styles
    );
    assert!(!has_static(&output, "margin", other));
    Ok(())
}

#[rstest]
#[case(
    "const base={space:'8px',browser:window.document};const alias=base;export const tokens={__proto__:alias,browser:window.document};"
)]
#[case(
    "import {base} from './base';const alias=base;export const tokens={__proto__:alias,browser:window.document};"
)]
#[serial]
fn demand_prototype_alias_emits_css_when_browser_siblings_are_unrelated(
    #[case] producer: &str,
) -> TestResult {
    // Given
    reset();
    let modules = [
        ("./prototype", "/prototype.ts", producer),
        (
            "./base",
            "/base.ts",
            "export const base={space:'8px',browser:window.document};throw new Error('unrelated base sibling');",
        ),
    ];
    // When
    let output = run("/alias.ts", CONSUMER, &modules)?;
    // Then
    assert!(has_static(&output, "margin", "8px"), "{:?}", output.styles);
    Ok(())
}

#[rstest]
#[case("export const tokens={__proto__:{space:window.document},browser:window.navigator};")]
#[case(
    "export const tokens={__proto__:{__proto__:{space:window.document}},browser:window.navigator};"
)]
#[serial]
fn demand_required_inherited_property_errors_when_its_value_is_unknown(
    #[case] producer: &str,
) -> TestResult {
    // Given
    reset();
    let offset = producer
        .find("window.document")
        .ok_or("fixture input missing")?;
    let place = crate::locate("/prototype.ts", producer, offset);
    // When
    let result = run(
        "/unknown.ts",
        CONSUMER,
        &[("./prototype", "/prototype.ts", producer)],
    );
    // Then
    let error = match result {
        Ok(output) => panic!(
            "required inherited value silently dropped: {} {:?}",
            output.code, output.styles
        ),
        Err(error) => error.to_string(),
    };
    assert!(error.contains(&place), "{error}");
    assert!(
        error.contains("cannot use `window.document` at build time"),
        "{error}"
    );
    assert!(error.contains("exact, static input"), "{error}");
    assert!(error.contains("Fix:"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn demand_imported_prototype_errors_at_its_owner_when_space_is_unknown() -> TestResult {
    // Given
    reset();
    let base = "export const base={space:window.document,browser:window.navigator};";
    let offset = base
        .find("window.document")
        .ok_or("fixture input missing")?;
    let place = crate::locate("/base.ts", base, offset);
    let modules = [
        (
            "./prototype",
            "/prototype.ts",
            concat!(
                "import ",
                "{base} from './base';export const tokens={__proto__:base,browser:window.navigator};"
            ),
        ),
        ("./base", "/base.ts", base),
    ];
    // When
    let result = run("/unknown-alias.ts", CONSUMER, &modules);
    // Then
    let error = match result {
        Ok(output) => panic!(
            "required imported value silently dropped: {} {:?}",
            output.code, output.styles
        ),
        Err(error) => error.to_string(),
    };
    assert!(error.contains(&place), "{error}");
    assert!(
        error.contains("cannot use `window.document` at build time"),
        "{error}"
    );
    assert!(error.contains("exact, static input"), "{error}");
    assert!(error.contains("Fix:"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn demand_omitted_method_does_not_activate_its_receiver_when_only_space_is_read() -> TestResult {
    // Given
    reset();
    let producer =
        "export const tokens={space:'8px',unused(){return this.browser;},browser:window.document};";
    // When
    let output = run(
        "/unused-method.ts",
        CONSUMER,
        &[("./prototype", "/prototype.ts", producer)],
    )?;
    // Then
    assert!(has_static(&output, "margin", "8px"), "{:?}", output.styles);
    Ok(())
}
