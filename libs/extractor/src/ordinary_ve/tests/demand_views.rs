use rstest::rstest;
use serial_test::serial;

use super::demand_support::{TestResult, assert_import, reset, run};
use super::mixed_support::{assert_consumed, has_static};

const TOKENS: &str = "export const PRIMARY='blue',browser=window.document;export function space(value){return `${value*2}px`;}throw new Error('unrelated token-module runtime sentinel');";
const OBJECT: &str = "export const tokens={color:'blue',space:'8px',browser:window.document};throw new Error('unrelated namespace runtime sentinel');";

#[rstest]
#[case("ts")]
#[case("tsx")]
#[case("js")]
#[case("jsx")]
#[case("mjs")]
#[case("css.ts")]
#[case("css.js")]
#[serial]
fn demand_helper_compiles_when_only_a_function_and_constant_are_required(
    #[case] suffix: &str,
) -> TestResult {
    // Given
    reset();
    let source = "import {style} from '@vanilla-extract/css';import {PRIMARY,space} from './tokens';export const box=style({color:PRIMARY,padding:space(4)});const browser=window.document;";
    // When
    let output = run(
        &format!("/helper.{suffix}"),
        source,
        &[("./tokens", "/tokens.ts", TOKENS)],
    )?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "color", "blue"));
    assert!(has_static(&output, "padding", "8px"));
    assert_import(&output, "./tokens");
    assert!(output.dependencies.iter().any(|path| path == "/tokens.ts"));
    Ok(())
}

#[rstest]
#[case("ts")]
#[case("tsx")]
#[case("js")]
#[case("jsx")]
#[case("mjs")]
#[serial]
fn demand_callable_keeps_callbacks_when_the_ordinary_producer_has_host_siblings(
    #[case] suffix: &str,
) -> TestResult {
    // Given
    reset();
    let path = format!("/callback.{suffix}");
    let producer = "export function apply(callback){return callback(4);}const browser=window.document;throw new Error('callback sibling');";
    let source = "import {style} from '@vanilla-extract/css';import {apply} from './callback';export const box=style({padding:apply(value=>`${value*2}px`)});";
    // When
    let output = run(
        "/callback-consumer.ts",
        source,
        &[("./callback", &path, producer)],
    )?;
    // Then
    assert!(has_static(&output, "padding", "8px"));
    Ok(())
}

#[rstest]
#[case(
    "export {PRIMARY as color,space} from './tokens';",
    "import {style} from '@vanilla-extract/css';import {color,space} from './barrel';export const box=style({color,padding:space(4)});"
)]
#[case(
    "export * from './tokens';",
    "import {style} from '@vanilla-extract/css';import * as ns from './barrel';export const box=style({color:ns.PRIMARY,padding:ns.space(4)});"
)]
#[case(
    "export * as palette from './tokens';",
    "import {style} from '@vanilla-extract/css';import {palette} from './barrel';export const box=style({color:palette.PRIMARY,padding:palette.space(4)});"
)]
#[serial]
fn demand_reexports_forward_only_required_names_when_the_barrel_has_runtime_effects(
    #[case] exports: &str,
    #[case] source: &str,
) -> TestResult {
    // Given
    reset();
    let barrel = format!("{exports}throw new Error('barrel runtime sentinel');");
    // When
    let output = run(
        "/reexport.ts",
        source,
        &[
            ("./barrel", "/barrel.ts", &barrel),
            ("./tokens", "/tokens.ts", TOKENS),
        ],
    )?;
    // Then
    assert!(has_static(&output, "color", "blue"));
    assert!(has_static(&output, "padding", "8px"));
    assert_import(&output, "./barrel");
    for path in ["/barrel.ts", "/tokens.ts"] {
        assert!(
            output
                .dependencies
                .iter()
                .any(|dependency| dependency == path)
        );
    }
    Ok(())
}

#[rstest]
#[case("import {tokens} from './object';", "tokens.space")]
#[case("import * as ns from './object';", "ns.tokens.space")]
#[serial]
fn demand_narrow_member_excludes_browser_when_a_namespace_or_named_object_is_read(
    #[case] import: &str,
    #[case] member: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';{import}export const box=style({{margin:{member}}});"
    );
    // When
    let output = run("/narrow.ts", &source, &[("./object", "/object.ts", OBJECT)])?;
    // Then
    assert!(has_static(&output, "margin", "8px"));
    Ok(())
}

#[rstest]
#[case("ts", false)]
#[case("tsx", false)]
#[case("js", false)]
#[case("jsx", false)]
#[case("mjs", false)]
#[case("css.ts", false)]
#[case("css.js", false)]
#[case("ts", true)]
#[serial]
fn demand_diamond_unions_property_paths_when_the_second_branch_needs_space(
    #[case] suffix: &str,
    #[case] reverse: bool,
) -> TestResult {
    // Given
    reset();
    let imports = if reverse {
        "import {space} from './space';import {color} from './color';"
    } else {
        "import {color} from './color';import {space} from './space';"
    };
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';{imports}export const box=style({{color,margin:space}});"
    );
    let modules = [
        ("./object", "/object.ts", OBJECT),
        (
            "./color",
            "/color.ts",
            "import {tokens} from './object';export const color=tokens.color;",
        ),
        (
            "./space",
            "/space.ts",
            "import {tokens} from './object';export const space=tokens.space;",
        ),
    ];
    // When
    let output = run(&format!("/diamond.{suffix}"), &source, &modules)?;
    // Then
    assert!(has_static(&output, "color", "blue"));
    assert!(has_static(&output, "margin", "8px"));
    for path in ["/object.ts", "/color.ts", "/space.ts"] {
        assert!(
            output
                .dependencies
                .iter()
                .any(|dependency| dependency == path)
        );
    }
    Ok(())
}

#[rstest]
#[case(
    "import {style} from '@vanilla-extract/css';import {tokens} from './object';export const box=style({content:Object.values(tokens).join(',')});"
)]
#[case(
    "import {style} from '@vanilla-extract/css';import * as ns from './object';export const box=style({content:JSON.stringify(ns)});"
)]
#[serial]
fn demand_whole_value_errors_when_observation_requires_the_browser_property(
    #[case] source: &str,
) -> TestResult {
    // Given
    reset();
    let offset = OBJECT
        .find("window.document")
        .ok_or("fixture browser missing")?;
    let place = crate::locate("/object.ts", OBJECT, offset);
    // When
    let result = run("/whole.ts", source, &[("./object", "/object.ts", OBJECT)]);
    // Then
    let error = match result {
        Ok(_) => panic!("required browser property was silently dropped"),
        Err(error) => error.to_string(),
    };
    assert!(error.contains(&place), "{error}");
    assert!(error.contains("window.document"), "{error}");
    assert!(error.contains("Fix:"), "{error}");
    Ok(())
}
