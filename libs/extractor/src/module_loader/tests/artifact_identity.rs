use super::artifacts::{active_values, exported};
use crate::{ExtractOption, ResolvedModule, extract_with_modules};
use css::{
    class_map::reset_class_map,
    file_map::{get_file_num_by_filename, reset_file_map},
};
use rstest::rstest;
use serial_test::serial;

const PRODUCER: &str = "import {style,createVar,createContainer,layer,fontFace,keyframes,createTheme} from '@devup-ui/react'; createVar(); fontFace({src:'local(x)'}); createContainer(); layer(); keyframes({to:{opacity:1}}); export const [theme,vars]=createTheme({space:'8px'}); export const base=style({color:'red',padding:8}); export const variants={nested:{base}};";
const CONSUMER: &str = "import {style,styleVariants} from '@devup-ui/react'; import * as p from './producer.css'; export const buttons=styleVariants({primary:'blue'},color=>[p.variants.nested.base,{color,margin:p.vars.space}]); export const button=buttons.primary;";

#[test]
#[serial]
fn serialization_keeps_native_schedule_when_getters_create_variables()
-> Result<(), Box<dyn std::error::Error>> {
    reset_file_map();
    let resolver = |_: &str, _: &str| None;
    let output = extract_with_modules(
        "/getter.css.ts",
        "import {style,createVar} from '@devup-ui/react'; export const base=style({get color(){createVar();return 'red'}}); export const space=createVar('space');",
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
        false,
        &resolver,
    )?;
    assert_eq!(exported(&output, "space")?, "var(--space-0-1)");
    assert_eq!(
        active_values(&output, &exported(&output, "base")?),
        vec![("color".into(), "red".into())]
    );
    Ok(())
}

#[rstest]
#[case(false)]
#[case(true)]
#[serial]
fn seeded_identity_is_stable_when_extraction_order_changes(
    #[case] producer_first: bool,
    #[values(false, true)] single_css: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    reset_file_map();
    reset_class_map();
    assert_eq!(get_file_num_by_filename("/producer.css.ts"), 0);
    assert_eq!(get_file_num_by_filename("/consumer.css.ts"), 1);
    let resolver = |_: &str, _: &str| {
        Some(ResolvedModule {
            path: "/producer.css.ts".into(),
            code: PRODUCER.into(),
        })
    };
    let option = ExtractOption {
        single_css,
        ..ExtractOption::default()
    };
    if producer_first {
        extract_with_modules(
            "/producer.css.ts",
            PRODUCER,
            option.clone(),
            false,
            &resolver,
        )?;
    }
    let output = extract_with_modules(
        "/consumer.css.ts",
        CONSUMER,
        option.clone(),
        false,
        &resolver,
    )?;
    let producer = extract_with_modules("/producer.css.ts", PRODUCER, option, false, &resolver)?;
    let classes = exported(&output, "button")?;
    let (bucket, global, _) = crate::resolve_css_target(
        "/consumer.css.ts",
        &ExtractOption {
            single_css,
            ..ExtractOption::default()
        },
    );
    let scope = (!global).then_some(bucket);
    let mut values: Vec<_> = output
        .styles
        .iter()
        .filter_map(|value| {
            let crate::ExtractStyleValue::Static(style) = value else {
                return None;
            };
            let Some(crate::extract_style::style_property::StyleProperty::ClassName(class)) =
                value.extract(scope.as_deref())
            else {
                return None;
            };
            classes
                .split_whitespace()
                .any(|token| token == class)
                .then(|| (style.property().to_string(), style.value().to_string()))
        })
        .collect();
    values.sort();
    assert_eq!(
        values,
        vec![
            ("color".into(), "blue".into()),
            ("margin".into(), "var(--space-0-5)".into()),
            ("padding".into(), "8px".into())
        ]
    );
    assert_eq!(exported(&producer, "theme")?, "theme-0-4");
    assert!(producer.code.contains("var(--space-0-5)"));
    assert!(
        producer
            .styles
            .iter()
            .any(|value| matches!(value, crate::ExtractStyleValue::FontFace(_)))
    );
    assert!(
        producer
            .styles
            .iter()
            .any(|value| matches!(value, crate::ExtractStyleValue::Keyframes(_)))
    );
    Ok(())
}

#[test]
#[serial]
fn dependency_edits_refresh_atoms_when_compiled_tokens_and_cache_keys_repeat()
-> Result<(), Box<dyn std::error::Error>> {
    let token = std::rc::Rc::new(std::cell::Cell::new("red"));
    let loaded_token = token.clone();
    let resolver = move |specifier: &str, _: &str| {
        match specifier {
        "./producer.css" => Some(ResolvedModule{path:"/cache-producer.css.ts".into(),code:"import {style} from '@devup-ui/react'; import {color} from './token'; export const base=style({color,padding:8});".into()}),
        "./forward.css" => Some(ResolvedModule{path:"/cache-forward.css.ts".into(),code:"import {createVar} from '@devup-ui/react'; import {base} from './producer.css'; createVar(); export const nested={base};".into()}),
        "./token" => Some(ResolvedModule{path:"/token.ts".into(),code:format!("export const color='{}';",loaded_token.get())}),
        _ => None,
    }
    };
    let code = "import {style} from '@devup-ui/react'; import {nested} from './forward.css'; export const button=style([nested.base,{ margin: 4 }]);";
    let mut observed = Vec::new();
    for color in ["red", "green"] {
        token.set(color);
        reset_file_map();
        reset_class_map();
        let output = extract_with_modules(
            "/cache-consumer.css.ts",
            code,
            ExtractOption {
                single_css: true,
                ..ExtractOption::default()
            },
            false,
            &resolver,
        )?;
        let classes = exported(&output, "button")?;
        observed.push(active_values(&output, &classes));
        assert!(output.dependencies.contains(&"/token.ts".into()));
    }
    assert_eq!(
        observed,
        vec![
            vec![
                ("color".into(), "red".into()),
                ("margin".into(), "4px".into()),
                ("padding".into(), "8px".into())
            ],
            vec![
                ("color".into(), "green".into()),
                ("margin".into(), "4px".into()),
                ("padding".into(), "8px".into())
            ]
        ]
    );
    Ok(())
}
