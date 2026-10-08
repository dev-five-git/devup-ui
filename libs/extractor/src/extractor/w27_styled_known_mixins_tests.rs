use crate::{ExtractOption, ExtractOutput, ExtractStyleValue, ImportAlias, ResolvedModule};
use std::collections::HashMap;

#[path = "w27_styled_known_mixins_regressions.rs"]
mod regressions;

fn option() -> ExtractOption {
    ExtractOption {
        single_css: true,
        import_aliases: HashMap::from([
            ("@emotion/react".to_string(), ImportAlias::NamedToNamed),
            (
                "@emotion/styled".to_string(),
                ImportAlias::DefaultToNamed("styled".to_string()),
            ),
        ]),
        ..ExtractOption::default()
    }
}

fn imported(base: &str, body: &str) -> ExtractOutput {
    let module = format!(
        "import styled from '@emotion/styled'; import {{ css }} from '@emotion/react'; {base}"
    );
    let entry =
        format!("import styled from '@emotion/styled'; import {{ Base }} from './base'; {body}");
    let resolver = move |specifier: &str, _: &str| {
        (specifier == "./base").then(|| ResolvedModule {
            path: "/w27/base.tsx".to_string(),
            code: module.clone(),
        })
    };
    crate::extract_with_modules("/w27/entry.tsx", &entry, option(), false, &resolver)
        .unwrap_or_else(|error| panic!("{error}"))
}

fn declarations(output: &ExtractOutput) -> Vec<(String, String)> {
    checked_declarations(output).unwrap_or_else(|error| panic!("{error}"))
}

fn checked_declarations(output: &ExtractOutput) -> Result<Vec<(String, String)>, String> {
    let mut styles: Vec<_> = output
        .styles
        .iter()
        .map(|value| match value {
            ExtractStyleValue::Static(style) => {
                Ok((style.property().to_string(), style.value().to_string()))
            }
            ExtractStyleValue::Typography(_)
            | ExtractStyleValue::Dynamic(_)
            | ExtractStyleValue::Css(_)
            | ExtractStyleValue::Import(_)
            | ExtractStyleValue::FontFace(_)
            | ExtractStyleValue::Keyframes(_) => {
                Err(format!("expected static atom, got {value:?}"))
            }
        })
        .collect::<Result<_, _>>()?;
    styles.sort();
    Ok(styles)
}

#[path = "w27_coverage_tests.rs"]
mod coverage_tests;

#[test]
#[serial_test::serial]
fn imported_extension_flattens_when_base_composes_known_object_mixin() {
    let base = "const mixin = css({ color: 'red', padding: 4 }); export const Base = styled.div`${mixin}; margin: 1px;`;";
    let output = imported(base, "export const Choice = styled(Base)`color: blue;`;");
    assert_eq!(
        declarations(&output),
        [
            ("color".into(), "blue".into()),
            ("margin".into(), "1px".into()),
            ("padding".into(), "4px".into())
        ]
    );
    assert!(output.code.contains("DevupAs = \"div\""));
    assert!(!output.code.contains("DevupAs = Base"));
    assert!(!output.code.contains("mixin"));
    assert!(output.code.contains("--"));
}

#[test]
#[serial_test::serial]
fn template_order_selects_color_when_known_tagged_mixin_is_first_or_last() {
    for (template, color) in [
        ("${mixin}; color: blue;", "blue"),
        ("color: blue; ${mixin};", "red"),
    ] {
        let base = format!(
            "const mixin = css`color: red; padding: 4px;`; export const Base = styled.div`{template}`;"
        );
        let output = imported(&base, "export const Choice = styled(Base)`margin: 1px;`;");
        assert_eq!(
            declarations(&output),
            [
                ("color".into(), color.into()),
                ("margin".into(), "1px".into()),
                ("padding".into(), "4px".into())
            ]
        );
    }
}

#[test]
#[serial_test::serial]
fn selectors_and_breakpoints_survive_when_imported_mixin_is_composed() {
    let base = "import { css as nativeCss } from '@devup-ui/react'; const mixin = nativeCss({ color: ['red', 'orange'], '&:hover': { color: 'red' } }); export const Base = styled.div`${mixin}; &:hover { color: blue; }`;";
    let output = imported(base, "export const Choice = styled(Base)`margin: 1px;`;");
    assert!(output.styles.iter().any(|value| matches!(value, ExtractStyleValue::Static(style) if style.level() == 1 && style.value() == "orange")));
    let hover: Vec<_> = output
        .styles
        .iter()
        .filter_map(|value| match value {
            ExtractStyleValue::Static(style)
                if style
                    .selector()
                    .is_some_and(|selector| selector.to_string().contains(":hover")) =>
            {
                Some(style.value())
            }
            _ => None,
        })
        .collect();
    assert_eq!(hover, ["blue"]);
    assert!(output.code.contains("DevupAs = \"div\""));
}

#[test]
#[serial_test::serial]
fn layered_metadata_keeps_later_atom_when_known_mixins_share_layer() {
    use crate::extract_style::extract_static_style::ExtractStaticStyle;
    use oxc_ast_visit::VisitMut;
    let allocator = oxc_allocator::Allocator::default();
    let source = "import { styled } from '@devup-ui/react'; import { first, second, theme } from './layers'; export const Choice = styled.div`${first}; ${theme}; ${second}; color: black;`;";
    let mut program = oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::tsx())
        .parse()
        .program;
    let mut visitor = crate::visit::DevupVisitor::new(
        &allocator,
        "w27-layers.tsx",
        "@devup-ui/react",
        vec![],
        None,
    );
    visitor.import_css(
        [
            ("first", "red", "base"),
            ("second", "blue", "base"),
            ("theme", "green", "theme"),
        ]
        .into_iter()
        .map(|(name, value, layer)| {
            (
                name.to_string(),
                vec![ExtractStyleValue::Static(
                    ExtractStaticStyle::new_with_layer(
                        "color",
                        value,
                        0,
                        None,
                        Some(layer.to_string()),
                    ),
                )],
            )
        })
        .collect(),
    );
    visitor.visit_program(&mut program);
    assert_eq!(visitor.errors, vec![]);
    let mut layers: Vec<_> = visitor
        .styles
        .iter()
        .map(|value| match value {
            ExtractStyleValue::Static(style) => {
                (style.layer().map(str::to_string), style.value().to_string())
            }
            other => panic!("expected static atom, got {other:?}"),
        })
        .collect();
    layers.sort();
    assert_eq!(
        layers,
        [
            (None, "black".into()),
            (Some("base".into()), "blue".into()),
            (Some("theme".into()), "green".into())
        ]
    );
}

#[test]
#[serial_test::serial]
fn inline_calls_and_tags_are_known_when_imported_base_composes_them() {
    for mixin in [
        "css({ color: 'red', padding: 4 })",
        "css`color: red; padding: 4px;`",
    ] {
        let base = format!("export const Base = styled.div`${{{mixin}}}; color: blue;`;");
        let output = imported(&base, "export const Choice = styled(Base)`margin: 1px;`;");
        assert_eq!(
            declarations(&output),
            [
                ("color".into(), "blue".into()),
                ("margin".into(), "1px".into()),
                ("padding".into(), "4px".into())
            ]
        );
    }
}

#[test]
#[serial_test::serial]
fn dynamic_values_keep_variables_when_known_mixin_joins_template() {
    let source = "import styled from '@emotion/styled'; import { css } from '@emotion/react'; const mixin = css({ padding: 4 }); export const Choice = styled.div`${mixin}; width: ${p => p.width};`;";
    let output = crate::extract("w27-dynamic-mixin.tsx", source, option())
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(output.styles.iter().any(
        |value| matches!(value, ExtractStyleValue::Dynamic(style) if style.property() == "width")
    ));
    assert!(
        !output
            .code
            .split("const Choice")
            .nth(1)
            .unwrap_or_default()
            .contains("mixin")
    );
    assert!(output.code.contains("--"));
}

#[test]
#[serial_test::serial]
fn runtime_class_stays_opaque_when_styles_are_unknown() {
    let source = "import styled from '@emotion/styled'; import { runtime } from './runtime'; export const Choice = styled.div`${runtime}; color: blue;`;";
    let output = crate::extract("w27-unknown-mixin.tsx", source, option())
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(output.code.contains("runtime"));
    assert_eq!(declarations(&output), [("color".into(), "blue".into())]);
}

#[test]
#[serial_test::serial]
fn runtime_class_remains_when_known_mixin_is_composed_beside_it() {
    let source = "import styled from '@emotion/styled'; import { css } from '@emotion/react'; import { runtime } from './runtime'; const mixin = css({ color: 'red' }); export const Choice = styled.div`${runtime}; ${mixin}; color: blue;`;";
    let output = crate::extract("w27-mixed-mixins.tsx", source, option())
        .unwrap_or_else(|error| panic!("{error}"));
    let choice = output.code.split("const Choice").nth(1).unwrap_or_default();
    assert!(choice.contains("runtime"));
    assert!(!choice.contains("mixin"));
}
