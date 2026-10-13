use rstest::rstest;
use serial_test::serial;

use super::demand_support::{TestResult, assert_import, global_selector, reset, run, static_value};
use super::mixed_support::{assert_consumed, has_static};

const PRODUCER: &str = "import {createTheme,style} from '@vanilla-extract/css';import {PRIMARY,space} from './tokens';import './reset.css';export const [theme,vars]=createTheme({space:space(4)});export const base=style({color:PRIMARY,padding:8});export const handler=()=>document.title;const browser=window.document;throw new Error('unrelated producer runtime sentinel');";
const CONTROL: &str = "import {createTheme,style,globalStyle} from '@vanilla-extract/css';export const [theme,vars]=createTheme({space:'8px'});export const base=style({color:'blue',padding:8});export const check=style({margin:vars.space});globalStyle(`${base}:focus`,{outlineColor:'purple'});";
const CONSUMER: &str = "import {style,globalStyle} from '@vanilla-extract/css';import {base,vars} from './producer';export const button=style([base,{color:'green',margin:vars.space}]);globalStyle(`${base}:focus`,{outlineColor:'purple'});const browser=window.document;";

#[rstest]
#[case("ts", "ts")]
#[case("tsx", "tsx")]
#[case("js", "js")]
#[case("jsx", "jsx")]
#[case("mjs", "mjs")]
#[case("css.ts", "css.ts")]
#[case("css.js", "css.js")]
#[serial]
fn demand_mixed_producer_keeps_exact_vars_d1_and_owner_selector_when_imported(
    #[case] consumer_suffix: &str,
    #[case] producer_suffix: &str,
) -> TestResult {
    // Given: an independently extracted producer supplies the identity oracle.
    reset();
    let path = format!("/producer.{producer_suffix}");
    let control = run(&path, CONTROL, &[])?;
    let margin = static_value(&control, "margin").to_string();
    let selector = global_selector(&control, "outline-color");
    let modules = [
        ("./producer", path.as_str(), PRODUCER),
        (
            "./tokens",
            "/tokens.ts",
            "export const PRIMARY='blue',browser=window.document;export function space(value){return `${value*2}px`;}throw new Error('unrelated token-module runtime sentinel');",
        ),
        ("./reset.css", "/reset.css", "body{margin:0}"),
    ];
    // When
    let output = run(&format!("/consumer.{consumer_suffix}"), CONSUMER, &modules)?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "color", "green"));
    assert!(!has_static(&output, "color", "blue"));
    assert!(has_static(&output, "padding", "8px"));
    assert!(has_static(&output, "margin", &margin));
    assert_eq!(global_selector(&output, "outline-color"), selector);
    assert!(has_static(&output, "outline-color", "purple"));
    assert_import(&output, "./producer");
    for dependency in [path.as_str(), "/tokens.ts", "/reset.css"] {
        assert!(output.dependencies.iter().any(|path| path == dependency));
    }
    assert!(output.code.contains("reset.css"), "{}", output.code);
    Ok(())
}

#[test]
#[serial]
fn demand_producer_keeps_unused_native_effects_when_only_vars_are_imported() -> TestResult {
    // Given
    reset();
    let producer = "import {createTheme,globalStyle} from '@vanilla-extract/css';export const [theme,vars]=createTheme({space:'8px'});\nglobalStyle('body',{color:window.name});\nthrow new Error('unrelated effect sibling');";
    let source = "import {style} from '@vanilla-extract/css';import {vars} from './producer';export const box=style({margin:vars.space});";
    let offset = producer
        .find("window.name")
        .ok_or("fixture effect missing")?;
    let place = crate::locate("/effect-producer.ts", producer, offset);
    // When
    let result = run(
        "/effect-consumer.ts",
        source,
        &[("./producer", "/effect-producer.ts", producer)],
    );
    // Then
    let error = match result {
        Ok(_) => panic!("unused native effect was dropped"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains(&place), "{error}");
    assert!(error.contains("window.name"), "{error}");
    assert!(error.contains("Fix:"), "{error}");
    assert!(!error.contains("unrelated effect sibling"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn demand_producer_initializes_once_when_two_branches_share_its_schedule() -> TestResult {
    // Given
    reset();
    let producer = "import {createVar,createTheme,style,globalStyle} from '@vanilla-extract/css';let runs=0;createVar();export const [theme,vars]=(()=>{runs++;return createTheme({space:'8px'});})();export const base=style({padding:8});globalStyle('body',{color:'purple'});export function count(){return runs;}const browser=window.document;throw new Error('schedule sibling');";
    let source = "import {style} from '@vanilla-extract/css';import {base,count} from './left';import {vars} from './right';export const box=style([base,{margin:vars.space,padding:`${count()}px`}]);";
    let control_source = format!("{producer}\nexport const check=style({{margin:vars.space}});");
    let control = run("/scheduled.ts", &control_source, &[])?;
    let margin = static_value(&control, "margin").to_string();
    assert_eq!(global_selector(&control, "color"), "body");
    assert!(has_static(&control, "color", "purple"));
    let modules = [
        ("./producer", "/scheduled.ts", producer),
        (
            "./left",
            "/left.ts",
            "export {base,count} from './producer';",
        ),
        ("./right", "/right.ts", "export {vars} from './producer';"),
    ];
    // When
    let output = run("/scheduled-consumer.ts", source, &modules)?;
    // Then: count() is an authored value, not a loader invocation count.
    assert!(has_static(&output, "padding", "1px"));
    assert!(has_static(&output, "margin", &margin));
    for edge in ["./left", "./right"] {
        assert_import(&output, edge);
    }
    assert!(
        output
            .dependencies
            .iter()
            .any(|path| path == "/scheduled.ts")
    );
    Ok(())
}
