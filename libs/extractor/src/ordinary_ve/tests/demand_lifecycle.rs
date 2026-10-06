use rstest::rstest;
use serial_test::serial;

use super::demand_support::{TestResult, global_selector, reset, run};
use super::mixed_support::has_static;

#[test]
#[serial]
fn demand_cycle_keeps_callable_closures_when_reads_wait_until_initialization() -> TestResult {
    // Given
    reset();
    let modules = [
        (
            "./even",
            "/even.ts",
            "import {odd} from './odd';export function even(n){return n===0?'blue':odd(n-1);}const browser=window.document;",
        ),
        (
            "./odd",
            "/odd.ts",
            "import {even} from './even';export function odd(n){return n===0?'red':even(n-1);}throw new Error('cycle sibling');",
        ),
    ];
    let source = "import {style} from '@vanilla-extract/css';import {even} from './even';export const box=style({color:even(4)});";
    // When
    let output = run("/cycle.ts", source, &modules)?;
    // Then
    assert!(has_static(&output, "color", "blue"));
    Ok(())
}

#[test]
#[serial]
fn demand_producer_atoms_refresh_when_the_resolved_source_changes() -> TestResult {
    // Given
    reset();
    let first = concat!(
        "import {style} from '@vanilla-extract/css';export const base=style(",
        "{padding:8});const browser=window.document;"
    );
    let changed = concat!(
        "import {style} from '@vanilla-extract/css';export const base=style(",
        "{padding:12});const browser=window.document;"
    );
    let source = "import {style,globalStyle} from '@vanilla-extract/css';import {base} from './producer';export const box=style([base,{color:'blue'}]);globalStyle(`${base}:focus`,{outlineColor:'purple'});";
    let warm = run(
        "/atom-consumer.ts",
        source,
        &[("./producer", "/atom-producer.ts", first)],
    )?;
    assert!(has_static(&warm, "padding", "8px"));
    // When
    let output = run(
        "/atom-consumer.ts",
        source,
        &[("./producer", "/atom-producer.ts", changed)],
    )?;
    // Then
    assert!(has_static(&output, "padding", "12px"));
    assert!(!has_static(&output, "padding", "8px"));
    assert_eq!(
        global_selector(&output, "outline-color"),
        global_selector(&warm, "outline-color")
    );
    Ok(())
}

#[test]
#[serial]
fn demand_cycle_errors_at_the_original_read_when_initialization_is_early() -> TestResult {
    // Given
    reset();
    let second = "import {first} from './first';\nexport const second=first;\nthrow new Error('unrelated cycle sentinel');";
    let modules = [
        (
            "./first",
            "/first.ts",
            concat!(
                "import ",
                "{second} from './second';export const first=second;"
            ),
        ),
        ("./second", "/second.ts", second),
    ];
    let offset = second.find("=first").ok_or("fixture read missing")? + 1;
    let place = crate::locate("/second.ts", second, offset);
    let source = concat!(
        "import {style} from '@vanilla-extract/css';import {first} from './first';export const box=style(",
        "{color:first});"
    );
    // When
    let result = run("/early.ts", source, &modules);
    // Then
    let error = match result {
        Ok(_) => panic!("early cyclic read succeeded"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains(&place), "{error}");
    assert!(
        error.contains(crate::module_loader::IMPORT_CYCLE),
        "{error}"
    );
    assert!(error.contains("Fix:"), "{error}");
    assert!(!error.contains("unrelated cycle sentinel"), "{error}");
    Ok(())
}

#[rstest]
#[case("import {color,change} from './state';", "color", "change()")]
#[case("import {color,change} from './barrel';", "color", "change()")]
#[case("import * as state from './state';", "state.color", "state.change()")]
#[serial]
fn demand_live_binding_reads_the_changed_value_when_a_required_helper_mutates_it(
    #[case] import: &str,
    #[case] color: &str,
    #[case] call: &str,
) -> TestResult {
    // Given
    reset();
    let modules = [
        (
            "./state",
            "/state.ts",
            "export let color='red';export function change(){color='blue';}const browser=window.document;throw new Error('live sibling');",
        ),
        (
            "./barrel",
            "/live-barrel.ts",
            "export {color,change} from './state';throw new Error('live barrel sibling');",
        ),
    ];
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';{import}export const box=(()=>{{{call};return style({{color:{color}}});}})();"
    );
    // When
    let output = run("/live.ts", &source, &modules)?;
    // Then
    assert!(has_static(&output, "color", "blue"));
    assert!(!has_static(&output, "color", "red"));
    Ok(())
}

#[test]
#[serial]
fn demand_views_are_fresh_when_a_second_consumer_and_changed_source_reuse_the_path() -> TestResult {
    // Given
    reset();
    let first = "export const tokens={color:'blue',space:'8px',browser:window.document};";
    let changed = "export const tokens={color:'green',space:'12px',browser:window.document};";
    let color_source = "import {style} from '@vanilla-extract/css';import {tokens} from './tokens';export const box=style({color:tokens.color});";
    let space_source = "import {style} from '@vanilla-extract/css';import {tokens} from './tokens';export const box=style({margin:tokens.space});";
    let warm = run(
        "/first.ts",
        color_source,
        &[("./tokens", "/fresh.ts", first)],
    )?;
    assert!(has_static(&warm, "color", "blue"));
    let second = run(
        "/second.ts",
        space_source,
        &[("./tokens", "/fresh.ts", first)],
    )?;
    assert!(has_static(&second, "margin", "8px"));
    // When
    let output = run(
        "/first.ts",
        color_source,
        &[("./tokens", "/fresh.ts", changed)],
    )?;
    // Then
    assert!(has_static(&output, "color", "green"));
    assert!(!has_static(&output, "color", "blue"));
    Ok(())
}

#[test]
#[serial]
fn demand_producer_metadata_is_fresh_when_the_class_prefix_changes() -> TestResult {
    // Given
    reset();
    let previous = css::get_prefix();
    let producer = concat!(
        "import {style} from '@vanilla-extract/css';export const base=style(",
        "{padding:8});const browser=window.document;"
    );
    let source = "import {style,globalStyle} from '@vanilla-extract/css';import {base} from './producer';export const box=style([base,{color:'blue'}]);globalStyle(`${base}:focus`,{outlineColor:'purple'});";
    let modules = [("./producer", "/prefix-producer.ts", producer)];
    css::set_prefix(Some("demand-before-".to_string()));
    let before = run("/prefix-consumer.ts", source, &modules);
    css::set_prefix(Some("demand-after-".to_string()));
    // When
    let after = run("/prefix-consumer.ts", source, &modules);
    css::set_prefix(previous);
    // Then
    let before = before?;
    let after = after?;
    assert!(has_static(&after, "padding", "8px"));
    assert!(has_static(&after, "color", "blue"));
    assert_eq!(
        global_selector(&before, "outline-color"),
        global_selector(&after, "outline-color")
    );
    assert!(before.code.contains("demand-before-"), "{}", before.code);
    assert!(after.code.contains("demand-after-"), "{}", after.code);
    assert!(!after.code.contains("demand-before-"), "{}", after.code);
    Ok(())
}
