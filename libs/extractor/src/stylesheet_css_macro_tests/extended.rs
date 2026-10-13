use super::*;

const DEFAULT_CSS_REFERENCE: &str =
    "import {css} from '@devup-ui/react';export const box=css({p:2});";

#[rstest]
#[case::later_native("[base,{p:3,color:'sienna'}]", "12px", "sienna")]
#[case::later_own("[{p:3,color:'sienna'},base]", "8px", "crimson")]
#[serial]
fn later_operand_wins_when_native_style_composes_own_css(
    #[case] operands: &str,
    #[case] padding: &str,
    #[case] color: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{css,style}} from '@devup-ui/react';const loop={{}};loop.self=loop;export {{loop}};const base=css({{p:2,color:'crimson'}});export const box=style({operands});"
    );
    let option = ExtractOption {
        single_css: true,
        ..ExtractOption::default()
    };
    // When
    let output = extract_without_source_map("/own-css-conflict.css.ts", &source, option)?;
    // Then
    let class = observe(&output, "exports.loop.self===exports.loop")?;
    assert_eq!(
        declarations(&output, &class),
        vec![
            ("color".into(), color.into()),
            ("padding".into(), padding.into())
        ]
    );
    Ok(())
}

#[rstest]
#[case::empty_global("globalCss({});")]
#[case::global("globalCss({body:{p:2}});")]
#[case::global_component("export const Global=createGlobalStyle({body:{p:2}});")]
#[serial]
fn own_global_macro_compiles_when_native_styles_and_cycles_require_execution(
    #[case] creation: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{css,style,globalCss,createGlobalStyle,globalStyle}} from '@devup-ui/react';const loop={{}};loop.self=loop;export {{loop}};{creation}const base=css({{p:2}});globalStyle('article',{{color:'rgba(1,2,3,0.5)'}});export const box=style([base,{{color:'sienna'}}]);"
    );
    let predicate = if creation.contains("Global=") {
        "exports.loop.self===exports.loop && typeof exports.Global==='function' && exports.Global()===null"
    } else {
        "exports.loop.self===exports.loop"
    };
    // When
    let output = extract_without_source_map(
        "/own-global.css.ts",
        &source,
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
    )?;
    // Then
    let class = observe(&output, predicate)?;
    assert_eq!(
        declarations(&output, &class),
        vec![
            ("color".into(), "sienna".into()),
            ("padding".into(), "8px".into())
        ]
    );
    assert!(output.styles.iter().any(|value| matches!(value,
        ExtractStyleValue::Static(style) if style.property() == "color" && style.value() == "#01020380"
            && matches!(style.selector(), Some(css::style_selector::StyleSelector::Global(selector, _)) if selector == "article")
    )), "{:?}", output.styles);
    let body_rules = output.styles.iter().filter(|value| matches!(value,
        ExtractStyleValue::Static(style) if style.property() == "padding" && style.value() == "8px"
            && matches!(style.selector(), Some(css::style_selector::StyleSelector::Global(selector, _)) if selector == "body")
    )).count();
    assert_eq!(body_rules, usize::from(creation != "globalCss({});"));
    Ok(())
}

#[test]
#[serial]
fn own_keyframes_keeps_normal_name_when_native_style_uses_it() -> TestResult {
    // Given
    let filename = "/own-keyframes.css.ts";
    let option = ExtractOption {
        single_css: true,
        ..ExtractOption::default()
    };
    reset();
    let reference = extract_without_source_map(
        filename,
        "import {keyframes} from '@devup-ui/react';export const box=keyframes({to:{opacity:1}});",
        option.clone(),
    )?;
    let name = observe(&reference, "true")?;
    reset();
    let source = "import {css,keyframes,style} from '@devup-ui/react';const loop={};loop.self=loop;export {loop};const spin=keyframes({to:{opacity:1}});const base=css({p:2});export const box=style([base,{animationName:spin,color:'sienna'}]);";
    // When
    let output = extract_without_source_map(filename, source, option)?;
    // Then
    let class = observe(&output, "exports.loop.self===exports.loop")?;
    assert_eq!(
        declarations(&output, &class),
        vec![
            ("animation-name".into(), name),
            ("color".into(), "sienna".into()),
            ("padding".into(), "8px".into())
        ]
    );
    assert_eq!(
        output
            .styles
            .iter()
            .filter(|value| matches!(value, ExtractStyleValue::Keyframes(_)))
            .count(),
        1
    );
    Ok(())
}

#[test]
#[serial]
fn expression_default_css_keeps_normal_class_when_fallback_runs() -> TestResult {
    // Given
    reset();
    let filename = "/own-default.css.ts";
    let option = ExtractOption {
        single_css: true,
        ..ExtractOption::default()
    };
    let reference = extract_without_source_map(filename, DEFAULT_CSS_REFERENCE, option.clone())?;
    let class = observe(&reference, "true")?;
    let predicate = format!(
        "exports.loop.self===exports.loop && exports.default==={}",
        serde_json::to_string(&class)?
    );
    reset();
    // When
    let output = extract_without_source_map(
        filename,
        "import {css,style} from '@devup-ui/react';const loop={};loop.self=loop;export {loop};export default css({p:2});export const box=style({color:'sienna'});",
        option,
    )?;
    // Then
    let box_class = observe(&output, &predicate)?;
    assert_eq!(
        declarations(&output, &class),
        vec![("padding".into(), "8px".into())]
    );
    assert_eq!(
        declarations(&output, &box_class),
        vec![("color".into(), "sienna".into())]
    );
    Ok(())
}

#[rstest]
#[case::throw("throw new Error('required import effect');", "required import effect")]
#[case::forbidden("try { Date.now(); } catch {}", "Date")]
#[serial]
fn required_import_effect_still_fails_when_own_css_compiles(
    #[case] effect: &str,
    #[case] cause: &str,
) -> TestResult {
    // Given
    reset();
    let helper = format!("export const color='sienna';\n{effect}");
    let resolver = move |specifier: &str, _: &str| {
        (specifier == "./effect").then(|| crate::ResolvedModule {
            path: "/own-effect.ts".into(),
            code: helper.clone(),
        })
    };
    let source = "import {css,style} from '@devup-ui/react';import {color} from './effect';const loop={};loop.self=loop;export {loop};const base=css({p:2});export const box=style([base,{color}]);";
    // When
    let result = crate::extract_with_modules(
        "/own-effect.css.ts",
        source,
        ExtractOption::default(),
        false,
        &resolver,
    );
    // Then
    let error = result
        .err()
        .ok_or("required effect was ignored")?
        .to_string();
    assert!(error.contains("/own-effect.ts:2:"), "{error}");
    assert!(error.contains(cause), "{error}");
    Ok(())
}

#[rstest]
#[case::removed("const seed=7;", "removed")]
#[case::rewritten("export const seed=7;", "rewritten")]
#[serial]
fn authored_read_keeps_original_location_when_own_macro_replacement_changes_length(
    #[case] seed: &str,
    #[case] cause: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "import {{css,style}} from '@devup-ui/react';\n{seed}\nconst base=css({{p:2}});\nexport const help=()=>seed;\nconst loop={{}};loop.self=loop;export {{loop}};\nexport const box=style([base,{{color:'sienna'}}]);"
    );
    // When
    let result =
        extract_without_source_map("/own-located.css.ts", &source, ExtractOption::default());
    // Then
    let error = result
        .err()
        .ok_or("unsafe authored read accepted")?
        .to_string();
    assert!(error.starts_with("/own-located.css.ts:4:23:"), "{error}");
    assert!(
        error.contains("`seed`") && error.contains(cause) && error.contains("Fix:"),
        "{error}"
    );
    Ok(())
}
