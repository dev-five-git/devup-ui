use rstest::rstest;
use serial_test::serial;

use super::consumer_support::{exact, modules, owner};
use super::demand_support::{TestResult, assert_import, reset, run, static_value};
use super::mixed_support::has_static;

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn shared_native_schedule_runs_once_when_devup_and_native_reads_are_unioned(
    #[case] native_entry: bool,
) -> TestResult {
    // Given
    reset();
    let producer = "import {createVar,createTheme,style,globalStyle} from '@vanilla-extract/css';let runs=0;createVar();export const [theme,vars]=(()=>{runs++;return createTheme({space:'8px'});})();export const base=style({padding:8});globalStyle('body',{color:'purple'});export const COLORS={fg:'blue',unused:window.document};export function count(){return runs;}throw new Error('schedule runtime only');";
    let control = "import {createVar,createTheme,style,globalStyle} from '@vanilla-extract/css';createVar();export const [theme,vars]=createTheme({space:'8px'});export const base=style({padding:8});globalStyle('body',{color:'purple'});export const check=style({margin:vars.space});";
    let expected = run("/scheduled.ts", control, &[])?;
    let margin = static_value(&expected, "margin").to_string();
    let extra = if native_entry {
        "import {style} from '@vanilla-extract/css';export const native=style({margin:vars.space});"
    } else {
        ""
    };
    let source = format!(
        "import {{css}} from '@devup-ui/react';import {{base,count,COLORS}} from './left';import {{vars}} from './right';{extra}export const box=css(base,{{color:COLORS.fg,margin:vars.space,padding:count()+'px'}});"
    );
    let graph = [
        ("./producer", "/scheduled.ts", producer),
        (
            "./left",
            "/left.ts",
            "export {base,count,COLORS} from './producer';",
        ),
        ("./right", "/right.ts", "export {vars} from './producer';"),
    ];
    // When
    let output = run("/union.ts", &source, &graph)?;
    // Then: count is authored state, not a resolver invocation count.
    assert!(has_static(&output, "padding", "1px"));
    assert!(!has_static(&output, "padding", "8px"));
    assert!(has_static(&output, "margin", &margin));
    assert!(has_static(&output, "color", "blue"));
    for edge in ["./left", "./right"] {
        assert_import(&output, edge);
    }
    for path in ["/scheduled.ts", "/left.ts", "/right.ts"] {
        assert!(
            output
                .dependencies
                .iter()
                .any(|dependency| dependency == path)
        );
    }
    assert!(!output.code.contains("count()"), "{}", output.code);
    Ok(())
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn readback_is_fresh_when_sources_change_after_either_producer_or_consumer_warmup(
    #[case] producer_first: bool,
) -> TestResult {
    // Given
    reset();
    let source = "import {css} from '@devup-ui/react';import {base,COLORS,vars} from './producer';export const box=css(base,{color:COLORS.fg,margin:vars.space});";
    let first = "import {createTheme,style} from '@vanilla-extract/css';export const [theme,vars]=createTheme({space:'8px'});export const base=style({padding:8});export const COLORS={fg:'blue',unused:window.document};";
    let changed = "import {createTheme,style} from '@vanilla-extract/css';export const [theme,vars]=createTheme({space:'12px'});export const base=style({padding:12});export const COLORS={fg:'green',unused:window.document};";
    if producer_first {
        run("/fresh.ts", first, &[])?;
    }
    let warm = run("/first.ts", source, &[("./producer", "/fresh.ts", first)])?;
    assert!(has_static(&warm, "color", "blue"));
    let second = run("/second.ts", source, &[("./producer", "/fresh.ts", first)])?;
    assert_eq!(
        static_value(&warm, "margin"),
        static_value(&second, "margin")
    );
    // When
    let output = run("/first.ts", source, &[("./producer", "/fresh.ts", changed)])?;
    // Then
    assert!(has_static(&output, "padding", "12px"));
    assert!(has_static(&output, "color", "green"));
    assert!(!has_static(&output, "padding", "8px"));
    assert!(!has_static(&output, "color", "blue"));
    assert_eq!(
        static_value(&warm, "margin"),
        static_value(&output, "margin")
    );
    Ok(())
}

#[test]
#[serial]
fn bridge_metadata_uses_current_prefix_when_a_previous_consumer_warmed_the_owner() -> TestResult {
    // Given
    reset();
    let previous = css::get_prefix();
    let source = "import {css} from '@devup-ui/react';import {base,vars} from './producer';export const box=css(base,{margin:vars.space});";
    let graph = modules("/producer.ts");
    css::set_prefix(Some("bridge-before-".to_string()));
    let before = run("/prefix.ts", source, &graph);
    css::set_prefix(Some("bridge-after-".to_string()));
    // When
    let after = run("/prefix.ts", source, &graph);
    css::set_prefix(previous);
    // Then
    let before = before?;
    let after = after?;
    assert!(has_static(&after, "padding", "8px"));
    assert_eq!(
        static_value(&before, "margin"),
        static_value(&after, "margin")
    );
    assert!(before.code.contains("bridge-before-"), "{}", before.code);
    assert!(after.code.contains("bridge-after-"), "{}", after.code);
    assert!(!after.code.contains("bridge-before-"), "{}", after.code);
    Ok(())
}

#[rstest]
#[case("{__proto__:{fg:'blue'},unused:window.document}", "COLORS.fg", "blue")]
#[case(
    "{__proto__:{fg:'red'},fg:'blue',unused:window.document}",
    "COLORS.fg",
    "blue"
)]
#[case(
    "{__proto__:{fg:'red'},['__proto__']:{fg:'blue'},unused:window.document}",
    "COLORS['__proto__'].fg",
    "blue"
)]
#[serial]
fn faithful_leaf_reads_preserve_inheritance_when_a_native_producer_activates_the_bridge(
    #[case] record: &str,
    #[case] read: &str,
    #[case] color: &str,
) -> TestResult {
    // Given
    reset();
    let expected = owner("/producer.ts")?;
    let producer =
        super::consumer_support::PRODUCER.replace("{fg:PRIMARY,unused:window.document}", record);
    let mut graph = modules("/producer.ts");
    graph[0].2 = &producer;
    let source = format!(
        "import {{css}} from '@devup-ui/react';import {{vars,COLORS}} from './producer';export const box=css({{margin:vars.space,color:{read}}});"
    );
    // When
    let output = run("/inherited.ts", &source, &graph)?;
    // Then
    exact(&output, &expected);
    assert!(has_static(&output, "color", color));
    assert!(!has_static(&output, "color", "red"));
    Ok(())
}
