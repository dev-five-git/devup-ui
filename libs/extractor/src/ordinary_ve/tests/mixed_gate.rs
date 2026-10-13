use rstest::rstest;
use serial_test::serial;

use super::mixed_canonical::assert_preserved;
use super::mixed_support::{assert_consumed, has_static};
use crate::{ExtractOption, ExtractOutput, ImportAlias, extract_with_modules};

fn extract(source: &str) -> Result<ExtractOutput, Box<dyn std::error::Error>> {
    css::file_map::reset_file_map();
    css::class_map::reset_class_map();
    extract_with_modules(
        "/primitive.tsx",
        source,
        ExtractOption {
            single_css: true,
            import_aliases: std::collections::HashMap::from_iter([
                ("@vanilla-extract/css".into(), ImportAlias::NamedToNamed),
                ("@emotion/react".into(), ImportAlias::NamedToNamed),
            ]),
            ..ExtractOption::default()
        },
        false,
        &|_, _| None,
    )
}

#[test]
#[serial]
fn primitive_native_classes_compose_when_an_ordinary_emotion_consumer_reads_them()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "import {style} from '@vanilla-extract/css';import {css} from '@emotion/react';const base=style({color:'red'});export const a=style([base,{margin:2}]);export const b=css(base,'extra',{padding:1});export const c=css([base]);const cond=true;export const d=style([cond&&base]);";
    // When
    let output = extract(source)?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "color", "red"));
    assert!(has_static(&output, "margin", "2px"));
    assert!(has_static(&output, "padding", "1px"));
    assert!(output.code.contains("extra"));
    Ok(())
}

#[test]
#[serial]
fn the_original_unknown_native_condition_is_fatal_after_primitive_consumers_are_allowed() {
    // Given
    let source = "import { style } from '@vanilla-extract/css';\nimport { css } from '@emotion/react';\nconst base = style({ color: 'red' });\nexport const a = style([base, { margin: 2 }]);\nexport const b = css(base, 'extra', { padding: 1 });\nexport const c = css([base]);\nexport const d = style([cond && base]);";
    let expected = crate::locate(
        "/primitive.tsx",
        source,
        source
            .find("cond")
            .unwrap_or_else(|| panic!("fixture condition missing")),
    );
    // When
    let result = extract(source);
    // Then
    let error = result
        .err()
        .unwrap_or_else(|| panic!("unknown native condition unexpectedly succeeded"))
        .to_string();
    assert!(error.contains(&expected), "{error}");
    assert!(error.contains("cond"), "{error}");
    assert!(error.contains("Fix:"), "{error}");
    assert!(!error.contains("may be changed"), "{error}");
}

#[rstest]
#[case(
    "const color='red';consume(color);export const base=style({color});",
    "consume(color);"
)]
#[case(
    "const tokens={color:'red'};consume(tokens.color);export const base=style(tokens);",
    "consume(tokens.color);"
)]
#[serial]
fn primitive_input_reads_are_safe_when_an_unselected_consumer_receives_them(
    #[case] body: &str,
    #[case] preserved: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = format!("import {{style}} from '@vanilla-extract/css';{body}");
    // When
    let output = extract(&source)?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "color", "red"));
    assert_preserved(&output.code, preserved);
    Ok(())
}

#[test]
#[serial]
fn mutable_native_contracts_are_rejected_when_an_unselected_consumer_can_change_them() {
    // Given
    let source = "import {createThemeContract,style} from '@vanilla-extract/css';export const vars=createThemeContract({color:null});const base=style({color:vars.color});consume(vars);";
    let expected = crate::locate(
        "/primitive.tsx",
        source,
        source
            .find("vars);")
            .unwrap_or_else(|| panic!("fixture escape missing")),
    );
    // When
    let result = extract(source);
    // Then
    let error = result
        .err()
        .unwrap_or_else(|| panic!("mutable escape unexpectedly succeeded"))
        .to_string();
    assert!(error.starts_with(&expected), "{error}");
    assert!(error.contains("may be changed"), "{error}");
    assert!(error.contains("Fix:"), "{error}");
}
