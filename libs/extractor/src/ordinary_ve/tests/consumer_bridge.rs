use rstest::rstest;
use serial_test::serial;

use super::consumer_support::{edges, exact, modules, owner};
use super::demand_support::{TestResult, assert_import, reset, run};
use super::mixed_support::has_static;

#[rstest]
#[case("ts", "css.ts")]
#[case("tsx", "ts")]
#[case("js", "css.js")]
#[case("jsx", "js")]
#[case("mjs", "tsx")]
#[case("css.ts", "jsx")]
#[case("css.js", "mjs")]
#[serial]
fn devup_only_css_uses_final_producer_data_when_suffixes_cross_native_boundaries(
    #[case] consumer: &str,
    #[case] producer: &str,
) -> TestResult {
    // Given: the independent control owns the exact native allocation schedule.
    reset();
    let path = format!("/producer.{producer}");
    let expected = owner(&path)?;
    let source = "import {css,globalCss} from '@devup-ui/react';import {base,vars,spin,COLORS} from './producer';export const box=css(base,{color:COLORS.fg,margin:vars.space,animationName:spin});globalCss({[`${base}:focus`]:{outlineColor:'purple'}});const browser=window.document;";
    // When
    let output = run(&format!("/consumer.{consumer}"), source, &modules(&path))?;
    // Then
    exact(&output, &expected);
    assert!(has_static(&output, "color", "blue"));
    assert!(!has_static(&output, "color", "red"));
    assert!(has_static(&output, "padding", "8px"));
    assert!(!has_static(&output, "padding", "32px"));
    assert!(has_static(&output, "animation-name", &expected.spin));
    assert_eq!(
        super::demand_support::global_selector(&output, "outline-color"),
        expected.base_selector
    );
    assert!(!output.code.contains("css("), "{}", output.code);
    assert!(output.code.contains("window.document"), "{}", output.code);
    edges(&output, &[&path, "/tokens.ts", "/reset.css"]);
    Ok(())
}

#[rstest]
#[case("tsx", "import {Box} from '@devup-ui/react';", "Box")]
#[case("jsx", "import {Box} from '@devup-ui/react';", "Box")]
#[case("tsx", "import {Box as Panel} from '@devup-ui/react';", "Panel")]
#[case("jsx", "import {Box as Panel} from '@devup-ui/react';", "Panel")]
#[case("tsx", "import * as UI from '@devup-ui/react';", "UI.Box")]
#[case("jsx", "import * as UI from '@devup-ui/react';", "UI.Box")]
#[serial]
fn devup_box_inlines_known_native_vars_when_handlers_children_and_runtime_values_survive(
    #[case] suffix: &str,
    #[case] import: &str,
    #[case] tag: &str,
) -> TestResult {
    // Given
    reset();
    let expected = owner("/producer.ts")?;
    let source = format!(
        "{import}import {{vars,COLORS}} from './producer';export const view=props=><{tag} m={{vars.space}} color={{COLORS.fg}} bg={{props.bg}} w={{vars.space+props.suffix}} onClick={{()=>document.title}}>{{window.name}}</{tag}>;"
    );
    // When
    let output = run(&format!("/box.{suffix}"), &source, &modules("/producer.ts"))?;
    // Then
    exact(&output, &expected);
    assert!(has_static(&output, "color", "blue"));
    assert!(output.code.contains("className="), "{}", output.code);
    assert!(!output.code.contains(&format!("<{tag}")), "{}", output.code);
    for runtime in [
        "props.bg",
        "props.suffix",
        "onClick=",
        "document.title",
        "window.name",
    ] {
        assert!(output.code.contains(runtime), "{}", output.code);
    }
    assert!(output.code.contains("style="), "{}", output.code);
    super::consumer_capture::box_values(&output, &expected.margin)?;
    edges(&output, &["/producer.ts", "/tokens.ts", "/reset.css"]);
    Ok(())
}

#[rstest]
#[case("import {vars as v} from './producer';", "v.space")]
#[case("import v from './producer';", "v.space")]
#[case("import * as p from './producer';", "p.vars.space")]
#[case("import * as p from './producer';const v=p.vars;", "v.space")]
#[case(concat!("import * as p from './producer';const {", "vars:v}=p;"), "v.space")]
#[case("import * as p from './producer';", "p['vars']['space']")]
#[serial]
fn devup_leaf_reads_keep_owner_values_when_data_bindings_are_aliased(
    #[case] binding: &str,
    #[case] read: &str,
) -> TestResult {
    // Given
    reset();
    let expected = owner("/producer.css.ts")?;
    let source = format!(
        "import {{css}} from '@devup-ui/react';{binding}export const box=css({{margin:{read}}});"
    );
    // When
    let output = run("/aliases.ts", &source, &modules("/producer.css.ts"))?;
    // Then
    exact(&output, &expected);
    assert!(
        !output.code.contains(&format!("margin: {read}")),
        "{}",
        output.code
    );
    edges(&output, &["/producer.css.ts"]);
    Ok(())
}

#[rstest]
#[case(
    "export {vars} from './producer';",
    "import {vars} from './barrel';",
    "vars.space"
)]
#[case(
    "export * from './producer';",
    "import {vars} from './barrel';",
    "vars.space"
)]
#[case(
    "export * as palette from './producer';",
    "import {palette} from './barrel';",
    "palette.vars.space"
)]
#[serial]
fn devup_barrel_reads_are_narrow_when_unrelated_barrel_effects_are_unknown(
    #[case] export: &str,
    #[case] import: &str,
    #[case] read: &str,
) -> TestResult {
    // Given
    reset();
    let expected = owner("/producer.ts")?;
    let barrel = format!(
        "{export}export const browser=window.document;throw new Error('barrel runtime only');"
    );
    let mut graph = modules("/producer.ts").to_vec();
    graph.push(("./barrel", "/barrel.ts", &barrel));
    let source = format!(
        "import {{css}} from '@devup-ui/react';{import}export const box=css({{margin:{read}}});"
    );
    // When
    let output = run("/barrel-consumer.ts", &source, &graph)?;
    // Then
    exact(&output, &expected);
    assert_import(&output, "./barrel");
    for path in ["/barrel.ts", "/producer.ts", "/tokens.ts", "/reset.css"] {
        assert!(
            output
                .dependencies
                .iter()
                .any(|dependency| dependency == path)
        );
    }
    Ok(())
}

#[test]
#[serial]
fn plain_literal_data_keeps_legacy_static_behavior_when_no_native_producer_exists() -> TestResult {
    // Given
    reset();
    // When
    let output = run(
        "/plain.ts",
        "import {css} from '@devup-ui/react';import {tokens} from './plain';export const box=css({margin:tokens.space,color:tokens.fg});",
        &[(
            "./plain",
            "/plain-data.ts",
            "export const tokens={space:'8px',fg:'blue'};",
        )],
    )?;
    // Then
    assert!(has_static(&output, "margin", "8px"));
    assert!(has_static(&output, "color", "blue"));
    assert_import(&output, "./plain");
    Ok(())
}

#[rstest]
#[case(
    "tsx",
    "import { Box } from '@devup-ui/react'\nimport { vars, PRIMARY } from './W35d-consumer-producer'\nexport const view = <Box p={vars.space} color={PRIMARY} />\n",
    "padding"
)]
#[case(
    "ts",
    "import { css } from '@devup-ui/react'\nimport { vars, PRIMARY } from './W35d-consumer-producer'\nexport const box = css({ margin: vars.space, color: PRIMARY })\n",
    "margin"
)]
#[serial]
fn actual_consumer_baselines_emit_static_native_vars_when_no_ve_api_is_imported(
    #[case] suffix: &str,
    #[case] source: &str,
    #[case] property: &str,
) -> TestResult {
    // Given
    reset();
    let producer = "import { createTheme, style } from '@vanilla-extract/css'\nexport const [theme, vars] = createTheme({ space: '8px' })\nexport const base = style({ padding: 8, color: 'red' })\nexport const PRIMARY = 'blue'\nconst browser = window.document\nthrow new Error('unrelated consumer producer sibling')\n";
    let control = "import {createTheme,style} from '@vanilla-extract/css';export const [theme,vars]=createTheme({space:'8px'});export const base=style({padding:8,color:'red'});export const check=style({margin:vars.space});";
    let independent = run("/W35d-consumer-producer.ts", control, &[])?;
    let expected = super::demand_support::static_value(&independent, "margin").to_string();
    assert_eq!(expected, "var(--space-0-1)");
    assert!(has_static(&independent, "--space-0-1", "8px"));
    // When
    let output = run(
        &format!("/W35d-consumer.{suffix}"),
        source,
        &[(
            "./W35d-consumer-producer",
            "/W35d-consumer-producer.ts",
            producer,
        )],
    )?;
    // Then
    assert!(has_static(&output, property, &expected));
    assert!(has_static(&output, "color", "blue"));
    assert!(!output.code.contains("vars.space"), "{}", output.code);
    assert!(!output.code.contains("style="), "{}", output.code);
    assert!(!output.code.contains("css("), "{}", output.code);
    assert_import(&output, "./W35d-consumer-producer");
    assert!(
        output
            .dependencies
            .iter()
            .any(|path| path == "/W35d-consumer-producer.ts")
    );
    Ok(())
}
