use crate::{ExtractOption, ResolvedModule, extract_with_modules, extract_without_source_map};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("css({ color: () => \"red\" })")]
#[case("css`color: ${() => \"red\"};`")]
#[case("css`padding: ${1n};`")]
#[serial]
fn css_tag_terminal_exact_public_probe_rejects_when_error_is_caught(
    #[case] invalid: &str,
    #[values("tsx", "css.ts")] suffix: &str,
) -> Result<(), String> {
    // Given: the exact parent public source, including its one-line catch mutation.
    let source = "import {css} from '@devup-ui/react'; const forceRun = (() => 1)(); try { "
        .to_string()
        + invalid
        + "; } catch (error) { error.message = 'ignored'; } export const card = css({p: 2});";
    let filename = format!("/src/css-edge.{suffix}");
    // When
    let error = extract_without_source_map(&filename, &source, ExtractOption::default())
        .err()
        .ok_or("caught unsupported css must remain terminal")?
        .to_string();
    // Then
    assert!(error.starts_with(&format!("{filename}:1:")), "{error}");
    assert!(error.contains("css()"), "{error}");
    if suffix == "css.ts" {
        assert!(error.starts_with(&format!("{filename}:1:74:")), "{error}");
        assert!(error.contains("Fix:"), "{error}");
    }
    assert_eq!(error.matches("ignored").count(), 0, "{error}");
    Ok(())
}

#[rstest]
#[case("import {css} from '@devup-ui/react';", "css")]
#[case("import {css as paint} from '@devup-ui/react';", "paint")]
#[case("import * as ui from '@devup-ui/react';", "ui.css")]
#[case(
    "import * as ui from '@devup-ui/react'; const ns=ui; const paint=ns.css;",
    "paint"
)]
#[serial]
fn css_tag_terminal_uses_authored_entry_or_helper_when_error_is_mutated(
    #[case] import: &str,
    #[case] tag: &str,
    #[values("() => 'red'", "1n")] value: &str,
    #[values(false, true)] imported: bool,
) -> Result<(), String> {
    // Given: the inner tag, not an enclosing make() call, owns the error.
    let body = format!(
        "{import}\nexport function make() {{\n  const marker = '한글';\n  try {{ {tag}`color: ${{{value}}};`; }} catch(error) {{ error.message='changed'; error.stack='changed'; error.name='changed'; }}\n  return {tag}({{p:2}});\n}}"
    );
    let entry = if imported {
        "import {css} from '@devup-ui/react'; import {make} from './helper'; const forceRun=(()=>1)(); export const card=make();".to_string()
    } else {
        format!("{body}\nconst forceRun=(()=>1)(); export const card=make();")
    };
    let resolver = move |_: &str, _: &str| {
        Some(ResolvedModule {
            path: "/helper.ts".into(),
            code: body.clone(),
        })
    };
    // When
    let error = extract_with_modules(
        "/entry.css.ts",
        &entry,
        ExtractOption::default(),
        false,
        &resolver,
    )
    .err()
    .ok_or("mutated unsupported tag must remain terminal")?
    .to_string();
    // Then
    let file = if imported {
        "/helper.ts"
    } else {
        "/entry.css.ts"
    };
    assert!(error.starts_with(&format!("{file}:4:9:")), "{error}");
    assert!(error.contains("css()"), "{error}");
    assert!(error.contains("Fix:"), "{error}");
    assert_eq!(error.matches("changed").count(), 0, "{error}");
    assert_eq!(error.matches("__devup_").count(), 0, "{error}");
    Ok(())
}

#[rstest]
#[case("() => 'red'")]
#[case("1n")]
#[serial]
fn css_tag_terminal_rejects_when_serialization_error_is_uncaught(
    #[case] value: &str,
) -> Result<(), String> {
    // Given
    let source = format!(
        "import {{css}} from '@devup-ui/react';\nconst forceRun=(()=>1)();\nexport const card=css`color: ${{{value}}};`;"
    );
    // When
    let error = extract_without_source_map("/uncaught.css.ts", &source, ExtractOption::default())
        .err()
        .ok_or("unsupported interpolation must fail")?
        .to_string();
    // Then
    assert!(error.starts_with("/uncaught.css.ts:3:"), "{error}");
    Ok(())
}

#[rstest]
#[serial]
fn css_tag_terminal_preserves_authored_and_native_frames_when_tagged_code_throws(
    #[values("ui.css`color:red;`", "ui.css('color:red;')")] invocation: &str,
) -> Result<(), String> {
    // Given
    let source = "import * as ui from '@devup-ui/react';\nconst forceRun=(()=>1)();\nui.css=function authored(parts){return [1].map(function explode(){throw new Error('tag failure');});};\nexport const card=".to_string() + invocation + ";";
    // When
    let error = extract_without_source_map("/frames.css.ts", &source, ExtractOption::default())
        .err()
        .ok_or("authored tag throw must fail")?
        .to_string();
    // Then
    assert!(error.starts_with("/frames.css.ts:3:"), "{error}");
    assert!(error.contains("at explode (/frames.css.ts:3:"), "{error}");
    assert!(error.contains("at <main> (/frames.css.ts:3:47)"), "{error}");
    assert!(error.contains("at map (native)"), "{error}");
    assert_eq!(error.matches(" (native)").count(), 1, "{error}");
    assert_eq!(error.matches("__vanilla_extract__").count(), 0, "{error}");
    Ok(())
}
