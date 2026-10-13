use rstest::rstest;
use serial_test::serial;

use super::consumer_support::owner;
use super::demand_support::{TestResult, assert_import, global_selector, reset, run, static_value};
use super::mixed_support::{assert_consumed, has_static};
use crate::ExtractStyleValue;

#[rstest]
#[serial]
fn carrier_preserves_root_owner_and_once_only_identifiers_when_a_diamond_demands_it(
    #[values("tsx", "css.ts", "css.js")] consumer_suffix: &str,
    #[values("css.ts", "css.js")] producer_suffix: &str,
) -> TestResult {
    // Given: the root-only extraction owns the expected native identities.
    reset();
    let producer_path = format!("/root-carrier.{producer_suffix}");
    let expected = owner(&producer_path)?;
    let roots = concat!(
        "import * as ve from '@vanilla-extract/css';import './reset.css';",
        "let runs=0;(runs++,ve.createVar());export const [theme,vars]=(runs++,ve.createTheme({space:'8px'}));",
        "export const base=(runs++,ve.style({color:'red',padding:8}));",
        "export const spin=(runs++,ve.keyframes({to:{opacity:1}}));(runs++,ve.globalStyle('body',{color:'purple'}));",
        "export function count(){return runs;}"
    );
    let control = run(
        &producer_path,
        roots,
        &[("./reset.css", "/reset.css", "body{margin:0}")],
    )?;
    let producer = format!(
        "{roots}{}",
        concat!(
            "const key='style';const make=ve[key];export {make};",
            "const browser=window.document;throw new Error('root sibling only');"
        )
    );
    let graph = [
        (
            "./left",
            "/root-left.css.ts",
            "export {make,base,count} from './origin';",
        ),
        (
            "./right",
            "/root-right.css.js",
            "export {vars,spin} from './origin';",
        ),
        ("./origin", producer_path.as_str(), producer.as_str()),
        ("./reset.css", "/reset.css", "body{margin:0}"),
    ];
    let source = concat!(
        "import {globalStyle} from '@vanilla-extract/css';import {make,base,count} from './left';",
        "import {vars,spin} from './right';export const box=make([base,{color:'blue',margin:vars.space,animationName:spin,zIndex:count()}]);",
        "globalStyle(`${base}:focus`,{outlineColor:'purple'});const browser=window.document;"
    );
    let variable = expected
        .margin
        .strip_prefix("var(")
        .and_then(|value| value.strip_suffix(')'))
        .ok_or("control variable missing")?;
    assert!(control.styles.iter().any(|value| matches!(value,
        ExtractStyleValue::Static(style) if style.property == "color" && style.value == "purple"
            && matches!(&style.selector, Some(css::style_selector::StyleSelector::Global(selector, _)) if selector == "body")
    )));
    assert_eq!(control.styles.iter().filter(|value| matches!(value,
        ExtractStyleValue::Static(style) if style.property == variable && style.value == "8px"
    )).count(), 1);
    assert_eq!(control.styles.iter().filter(|value| matches!(value,
        ExtractStyleValue::Static(style) if style.property.starts_with("--space-") && style.value == "8px"
    )).count(), 1);
    assert_eq!(control.styles.iter().filter(|value| matches!(value,
        ExtractStyleValue::Static(style) if style.property == "color" && style.value == "purple"
    )).count(), 1);
    assert_eq!(
        control
            .styles
            .iter()
            .filter(|value| matches!(value, ExtractStyleValue::Keyframes(_)))
            .count(),
        1
    );
    assert!(control.styles.iter().any(|value| matches!(value,
        ExtractStyleValue::Keyframes(frames) if frames.keyframes.get("to").is_some_and(|styles|
            styles.iter().any(|style| style.property == "opacity" && style.value == "1"))
    )));
    // When
    let output = run(&format!("/root-entry.{consumer_suffix}"), source, &graph)?;
    // Then: producer effects stay producer-owned; identities and the finite count are consumed.
    assert_consumed(&output);
    assert!(has_static(&output, "color", "blue"));
    assert!(has_static(&output, "padding", "8px"));
    assert!(!has_static(&output, "padding", "32px"));
    assert_eq!(static_value(&output, "margin"), expected.margin);
    assert_eq!(static_value(&output, "animation-name"), expected.spin);
    assert_eq!(
        global_selector(&output, "outline-color"),
        expected.base_selector
    );
    assert!(has_static(&output, "outline-color", "purple"));
    assert!(has_static(&output, "z-index", "5"));
    assert!(!output.code.contains("make("));
    assert!(output.code.contains("window.document"));
    assert!(output.code.contains("reset.css"));
    for import in ["./left", "./right"] {
        assert_import(&output, import);
    }
    for dependency in [
        producer_path.as_str(),
        "/root-left.css.ts",
        "/root-right.css.js",
        "/reset.css",
    ] {
        assert!(output.dependencies.iter().any(|path| path == dependency));
    }
    Ok(())
}
