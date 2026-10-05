use super::artifacts::{active_values, exported};
use crate::{ExtractOption, ExtractStyleValue, ResolvedModule, extract_with_modules};
use css::file_map::reset_file_map;
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case(
    "selectors:{'&:hover':{color:'red'}}",
    "selectors:{'&:hover':{color:'blue'}}"
)]
#[case(
    "'@media':{'print':{color:'red'}}",
    "'@media':{'print':{color:'blue'}}"
)]
#[case(
    "'@supports':{'(display: grid)':{color:'red'}}",
    "'@supports':{'(display: grid)':{color:'blue'}}"
)]
#[case(
    "'@container':{'(min-width: 10px)':{color:'red'}}",
    "'@container':{'(min-width: 10px)':{color:'blue'}}"
)]
#[case("'@layer':{'base':{color:'red'}}", "'@layer':{'base':{color:'blue'}}")]
#[case("vars:{'--custom':'red'}", "vars:{'--custom':'blue'}")]
#[case("color:['red',null,'red']", "color:['blue',null,'blue']")]
#[serial]
fn matching_dimensions_override_when_imported_rules_compose(
    #[case] base: &str,
    #[case] later: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    reset_file_map();
    let producer =
        format!("import {{style}} from '@devup-ui/react'; export const base=style({{{base}}});");
    let resolver = move |_: &str, _: &str| {
        Some(ResolvedModule {
            path: "/dimensions.css.ts".into(),
            code: producer.clone(),
        })
    };
    let output = extract_with_modules(
        "/consumer.css.ts",
        &format!(
            "import {{style}} from '@devup-ui/react'; import {{base}} from './dimensions.css'; export const button=style([base,{{{later}}}]);"
        ),
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
        false,
        &resolver,
    )?;
    let classes = exported(&output, "button")?;
    let values = active_values(&output, &classes);
    assert_ne!(values.len(), 0);
    assert!(
        values.iter().all(|(_, value)| value == "blue"),
        "{values:?}"
    );
    Ok(())
}

#[test]
#[serial]
fn disjoint_dimensions_coexist_when_inline_nested_objects_repeat()
-> Result<(), Box<dyn std::error::Error>> {
    reset_file_map();
    let resolver = |_: &str, _: &str| None;
    let output = extract_with_modules(
        "/nested.css.ts",
        "import {style} from '@devup-ui/react'; export const button=style([{selectors:{'&:hover':{color:'red',padding:8}},'@media':{print:{margin:4}}},{selectors:{'&:hover':{color:'blue'}},'@media':{print:{color:'green'}}}]);",
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
        false,
        &resolver,
    )?;
    let classes = exported(&output, "button")?;
    assert_eq!(
        active_values(&output, &classes),
        vec![
            ("color".into(), "blue".into()),
            ("color".into(), "green".into()),
            ("margin".into(), "4px".into()),
            ("padding".into(), "8px".into())
        ]
    );
    assert!(
        output
            .styles
            .iter()
            .filter_map(|value| match value {
                ExtractStyleValue::Static(style) => Some(style),
                _ => None,
            })
            .any(|style| style.selector().is_some())
    );
    Ok(())
}

#[test]
#[serial]
fn selector_anchors_survive_when_known_atoms_are_overridden()
-> Result<(), Box<dyn std::error::Error>> {
    reset_file_map();
    let resolver = |_: &str, _: &str| {
        Some(ResolvedModule {path:"/anchor.css.ts".into(),code:"import {style,globalStyle} from '@devup-ui/react'; export const base=style([{color:'red'},'external']); globalStyle(`${base} > span`,{padding:8});".into()})
    };
    let output = extract_with_modules(
        "/consumer.css.ts",
        "import {style} from '@devup-ui/react'; import {base} from './anchor.css'; export const button=style([base,{color:'blue'}]);",
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
        false,
        &resolver,
    )?;
    let classes = exported(&output, "button")?;
    assert_eq!(
        active_values(&output, &classes),
        vec![("color".into(), "blue".into())]
    );
    assert!(classes.split_whitespace().any(|token| token == "external"));
    assert!(classes.split_whitespace().any(|token| token == "f0_base"));
    Ok(())
}
