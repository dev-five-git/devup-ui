use css::style_selector::StyleSelector;
use rstest::rstest;
use serial_test::serial;

use crate::{ExtractOption, ExtractStyleValue, ImportAlias, ResolvedModule, extract_with_modules};

#[rstest]
#[case(r#"button[data-label="escaped\"quote"] "#)]
#[case(r"button[data-label='escaped\'quote'] ")]
#[case(r"button\:hover ")]
#[serial]
fn imported_class_identity_when_selector_contains_escapes_keeps_the_escaped_text(
    #[case] prefix: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    css::file_map::reset_file_map();
    css::class_map::reset_class_map();
    let source = format!(
        "import {{globalStyle}} from '@vanilla-extract/css';import {{base}} from './base.css';globalStyle({}+base,{{margin:7}});",
        serde_json::to_string(prefix)?
    );
    let output = extract_with_modules(
        "/escapes.css.ts",
        &source,
        ExtractOption {
            single_css: true,
            import_aliases: std::collections::HashMap::from_iter([(
                "@vanilla-extract/css".into(),
                ImportAlias::NamedToNamed,
            )]),
            ..ExtractOption::default()
        },
        false,
        &|specifier, _| {
            (specifier == "./base.css").then(|| ResolvedModule {
                path: "/base.css.ts".into(),
                code: "import {style} from '@vanilla-extract/css';export const base=style({color:'red'});".into(),
            })
        },
    )?;
    let globals: Vec<_> = output
        .styles
        .iter()
        .filter_map(|value| {
            if let ExtractStyleValue::Static(style) = value
                && style.property() == "margin"
                && let Some(StyleSelector::Global(selector, file)) = &style.selector
            {
                Some((selector.clone(), file.clone(), style.value().to_string()))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(
        globals,
        vec![(
            format!("{prefix}.f0_0"),
            "/escapes.css.ts".into(),
            "7px".into()
        )]
    );
    Ok(())
}
