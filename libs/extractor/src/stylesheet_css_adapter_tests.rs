use std::collections::BTreeMap;

use crate::{ExtractOption, ExtractOutput, ExtractStyleValue, extract_without_source_map};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

fn compile(source: &str) -> Result<ExtractOutput, String> {
    reset_class_map();
    reset_file_map();
    extract_without_source_map(
        "adapter.css.ts",
        source,
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
    )
    .map_err(|error| error.to_string())
}

fn bindings(code: &str) -> BTreeMap<&str, &str> {
    code.lines()
        .filter_map(|line| {
            line.trim_start_matches("export ")
                .strip_prefix("const ")?
                .trim_end_matches(';')
                .split_once(" = ")
        })
        .filter(|(name, _)| *name != "forceRun")
        .collect()
}

#[rstest]
#[case("export const card = css({ p:2,w:2,opacity:0.5 });")]
#[case("const base = css({p:2,color:'red'}); export const card = css(base,{p:3,color:'blue'});")]
#[case("const zebra = css({p:2,color:'red'}); export const alpha = css(zebra,{p:3,color:'blue'});")]
#[case("export const zebra = css({ p:2 }); export const alpha = css({color:'blue'});")]
#[case("export const card = css({p:[1,null,2],_hover:{p:[2,null,3]},selectors:{'& > a':{w:2}}});")]
#[case(
    "const zebra = css({p:[1,2],_hover:{color:'red',p:[2,3]}}); export const alpha = css([zebra,[{p:[3,null],_hover:{color:'blue'}}]],{ p:4 });"
)]
#[case("export const card = css({ p:2 },{p:3,color:'blue'});")]
#[case("export const card = css([{ p:2 },[{p:3,color:'blue'}]]);")]
#[case("const base = css({p:2,color:'red'}); export const card = css({p:3},base,{color:'blue'});")]
#[case(
    "const base = css({ p:2 }); const next = css(base,{ p:3 }); export const card = css(next,base);"
)]
#[case("export const card = css('padding: 2px; color: red; &:hover { opacity: .5; }');")]
#[case("export const card = css`padding: ${2}px; color: ${'red'}; &:hover { opacity: .5; }`;")]
#[case("export const card = css({p:2,opacity:0.5,styleOrder:2,_hover:{color:'red'}});")]
#[case("export const card = css(null,false,undefined,{ p:2 });")]
#[case("export const card = css();")]
#[case("export const card = css({});")]
#[case("export const card = css({p:2,_hover:{ p:1 }},{p:3,_hover:{ p:2 }});")]
#[case(
    "const base = css({p:1,'@layer':{base:{color:'red'}}}); export const card = css(base,{'@layer':{base:{color:'blue'}}});"
)]
#[case("const base = css({ p:2 }); export const card = css(...[base,{ p:3 }]);")]
#[serial]
fn full_css_adapter_matches_classes_and_styles_when_forced_to_run(
    #[case] body: &str,
) -> Result<(), String> {
    // Given: the same supported source, with only an unrelated effect forcing Run.
    let source = format!("import {{css}} from '@devup-ui/react'; {body}");
    assert_eq!(
        crate::stylesheet_policy::plan(
            &source,
            "adapter.css.ts",
            &ExtractOption::default(),
            None,
            &|_| false
        ),
        crate::stylesheet_policy::Plan::Plain
    );
    let expected = compile(&source)?;
    let full =
        format!("import {{css}} from '@devup-ui/react'; const forceRun = (() => 1)(); {body}");
    assert_eq!(
        crate::stylesheet_policy::plan(
            &full,
            "adapter.css.ts",
            &ExtractOption::default(),
            None,
            &|_| false
        ),
        crate::stylesheet_policy::Plan::Run
    );
    // When
    let actual = compile(&full)?;
    // Then: class identity is part of parity, not just equivalent declarations.
    assert_eq!(bindings(&actual.code), bindings(&expected.code), "{body}");
    assert_eq!(actual.styles, expected.styles, "{body}");
    assert_eq!(actual.code.matches("var(--").count(), 0);
    Ok(())
}

#[rstest]
#[case("import {css as make} from '@devup-ui/react';", "make")]
#[case("import * as own from '@devup-ui/react';", "own.css")]
#[serial]
fn full_css_adapter_preserves_import_identity_when_aliased_or_namespaced(
    #[case] import: &str,
    #[case] callee: &str,
) -> Result<(), String> {
    // Given
    let body = format!("export const card = {callee}({{p:2,opacity:0.5}});");
    let expected = compile(&format!("{import} {body}"))?;
    // When
    let actual = compile(&format!("{import} const forceRun = (() => 1)(); {body}"))?;
    // Then
    assert_eq!(bindings(&actual.code), bindings(&expected.code));
    assert_eq!(actual.styles, expected.styles);
    Ok(())
}

#[rstest]
#[case("String(new Date(0).getTime())", "0")]
#[case("String(Date.UTC(2020,0,1))", "1577836800000")]
#[serial]
fn full_css_adapter_is_static_when_date_capability_returns_an_approved_number(
    #[case] expression: &str,
    #[case] value: &str,
) -> Result<(), String> {
    // Given
    let expected = compile(&format!(
        "import {{css}} from '@devup-ui/react'; export const card = css({{opacity:'{value}'}});"
    ))?;
    // When
    let actual = compile(&format!(
        "import {{css}} from '@devup-ui/react'; export const card = css({{opacity:{expression}}});"
    ))?;
    // Then
    assert_eq!(bindings(&actual.code), bindings(&expected.code));
    assert_eq!(actual.styles, expected.styles);
    assert_eq!(actual.code.matches("Date").count(), 0);
    Ok(())
}

#[test]
#[serial]
fn full_css_adapter_keeps_devup_scaling_when_mixed_with_vanilla_style() -> Result<(), String> {
    // Given
    let source = "import {css,style} from '@devup-ui/react'; export const vanilla = style({padding:2,opacity:0.5}); export const own = css({p:2,opacity:0.5});";
    // When
    let actual = compile(source)?;
    // Then
    let declarations: Vec<_> = actual
        .styles
        .iter()
        .filter_map(|value| match value {
            ExtractStyleValue::Static(style) => Some((style.property(), style.value())),
            _ => None,
        })
        .collect();
    assert!(
        declarations.contains(&("padding", "2px")),
        "{declarations:?}"
    );
    assert!(
        declarations.contains(&("padding", "8px")),
        "{declarations:?}"
    );
    let opacity = declarations
        .iter()
        .find(|(property, _)| *property == "opacity")
        .ok_or("opacity declaration must remain unitless")?
        .1
        .parse::<f64>()
        .map_err(|error| error.to_string())?;
    assert!((opacity - 0.5).abs() < f64::EPSILON, "{declarations:?}");
    Ok(())
}

#[rstest]
#[case(
    "const fade = keyframes({from:{opacity:0},to:{opacity:1}}); export const card = css({animation:`${fade} 1s`});"
)]
#[case(
    "const zebra = css({ p:2 }); const fade = keyframes({from:{opacity:0},to:{opacity:1}}); export const card = css(zebra,{animation:`${fade} 1s`});"
)]
#[case(
    "const zebra = keyframes({from:{opacity:0}}); const alpha = keyframes({from:{opacity:1}}); export const card = css({animation:`${zebra} 1s, ${alpha} 2s`});"
)]
#[serial]
fn full_css_adapter_preserves_keyframe_values_when_interpolated_in_own_rules(
    #[case] body: &str,
) -> Result<(), String> {
    // Given
    let expected = compile(&format!(
        "import {{css,keyframes}} from '@devup-ui/react'; {body}"
    ))?;
    // When
    let actual = compile(&format!(
        "import {{css,keyframes}} from '@devup-ui/react'; const forceRun=(()=>1)(); {body}"
    ))?;
    // Then
    assert_eq!(bindings(&actual.code), bindings(&expected.code));
    assert_eq!(actual.styles, expected.styles);
    Ok(())
}

#[test]
#[serial]
fn full_css_adapter_does_not_capture_shadowed_ordinary_css_helpers() -> Result<(), String> {
    // Given
    let source = "import {css as own} from '@devup-ui/react'; function css(){return 'blue'} export const card=own({color:css(),p:2});";
    let expected =
        compile("import {css} from '@devup-ui/react'; export const card=css({color:'blue',p:2});")?;
    // When
    let actual = compile(source)?;
    // Then
    assert_eq!(bindings(&actual.code), bindings(&expected.code));
    assert_eq!(actual.styles, expected.styles);
    Ok(())
}

#[test]
#[serial]
fn full_css_adapter_reads_rule_getters_once_when_serializing_static_values() -> Result<(), String> {
    // Given
    let source = "import {css} from '@devup-ui/react'; let reads=0; export const card=css({get p(){reads++;return 2},opacity:0.5}); export const count=reads;";
    // When
    let actual = compile(source)?;
    // Then
    assert_eq!(bindings(&actual.code).get("card"), Some(&"\"a b\""));
    assert_eq!(bindings(&actual.code).get("count"), Some(&"1"));
    Ok(())
}

#[rstest]
#[case(
    "const base=css({p:2,color:'red'}); export const card=style([base,{padding:3,color:'blue'}]);",
    "const base=css({p:2,color:'red'}); export const card=css(base,{padding:'3px',color:'blue'});"
)]
#[case(
    "const base=style({padding:2,color:'red'}); export const card=css(base,{p:3,color:'blue'});",
    "const base=css({padding:'2px',color:'red'}); export const card=css(base,{p:3,color:'blue'});"
)]
#[case(
    "const base=css({p:2,_hover:{color:'red'}}); export const card=style([base,{color:'blue'}]);",
    "const base=css({p:2,_hover:{color:'red'}}); export const card=css(base,{color:'blue'});"
)]
#[serial]
fn full_css_adapter_preserves_d1_when_own_and_vanilla_styles_compose(
    #[case] body: &str,
    #[case] normalized: &str,
) -> Result<(), String> {
    // Given
    let expected = compile(&format!(
        "import {{css}} from '@devup-ui/react'; {normalized}"
    ))?;
    // When
    let actual = compile(&format!(
        "import {{css,style}} from '@devup-ui/react'; {body}"
    ))?;
    // Then
    assert_eq!(bindings(&actual.code), bindings(&expected.code));
    assert_eq!(actual.styles, expected.styles);
    Ok(())
}
