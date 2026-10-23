use super::*;
use crate::ExtractStyleValue;

fn rejection(result: Result<crate::ExtractOutput, Box<dyn std::error::Error>>) -> String {
    match result {
        Ok(output) => panic!("expected located refusal: {output:?}"),
        Err(error) => error.to_string(),
    }
}

fn options(single_css: bool) -> ExtractOption {
    ExtractOption {
        single_css,
        import_aliases: std::collections::HashMap::from([
            (
                "@emotion/react".to_string(),
                crate::ImportAlias::NamedToNamed,
            ),
            (
                "@emotion/styled".to_string(),
                crate::ImportAlias::DefaultToNamed("styled".to_string()),
            ),
        ]),
        ..ExtractOption::default()
    }
}

#[rstest]
#[case::root_changed("let rules={color:'blue'};rules.color='green';", "")]
#[case::root_unknown("const rules=getRules();", "")]
#[case::sibling("function other(){const rules={color:'blue'};observe(rules);}", "")]
#[case::shadowed_api(
    "",
    "function other(css){const rules={color:'blue'};return css(rules);}"
)]
#[serial]
fn local_demand_stays_separate_when_module_names_overlap(
    #[case] before: &str,
    #[case] after: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{css as make}} from '@devup-ui/react';{before}export function f(){{const rules={{color:'red'}};return make(rules);}}{after}"
    );
    // When
    let output = extract("intersection.tsx", &source, options(single_css))
        .unwrap_or_else(|error| panic!("{error}"));
    // Then
    let colors: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property() == "color" => Some(style.value()),
            _ => None,
        })
        .collect();
    assert_eq!(colors, ["red"]);
}

#[rstest]
#[case::template("const holder={rules:{color:'red'}};", "make`${holder.rules}`")]
#[case::selected_template(
    "const holder={rules:{color:'red'},other:{color:'blue'}};",
    "make`${true?holder.rules:holder.other}`"
)]
#[case::class_names(
    "const holder={rules:{color:'red'}};",
    "<ClassNames>{({css:paint})=><div className={paint(holder.rules)}/>}</ClassNames>"
)]
#[serial]
fn erased_local_reads_keep_extra_use_errors_when_templates_or_class_names_consume_them(
    #[case] declaration: &str,
    #[case] expression: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{css as make}} from '@devup-ui/react';import {{ClassNames}} from '@emotion/react';\nexport function f(){{{declaration}observe(holder);return {expression};}}"
    );
    // When
    let result = extract("intersection.tsx", &source, options(single_css));
    // Then
    let error = rejection(result);
    assert!(error.starts_with("intersection.tsx:2:27:"), "{error}");
    assert!(
        error.contains(&crate::utils::local_style_error("holder")),
        "{error}"
    );
}

#[rstest]
#[case::leaf("const Card=styled.div(p=>({ color: alias }));")]
#[case::default("const Card=styled.div((p=alias)=>({color:p.color}));")]
#[case::controller("const Card=styled.div(alias?[{color:'red'}]:[{color:'blue'}]);")]
#[case::attrs("const Card=styled.div.attrs({ title: alias })({color:'red'});")]
#[serial]
fn captured_reads_are_located_when_the_original_alias_is_removed(
    #[case] body: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!("import {{css,styled}} from '@devup-ui/react';\nconst alias=css;{body}");
    // When
    let result = extract("intersection.tsx", &source, options(single_css));
    // Then
    let error = rejection(result);
    assert!(error.contains("intersection.tsx:2:"), "{error}");
    assert!(error.contains("`alias` is read at runtime"), "{error}");
}

#[rstest]
#[case::shadowed_leaf("const Card=styled.div(alias=>({color:alias.color}));")]
#[case::shadowed_attrs(
    "function f(alias){return styled.div.attrs({ title: alias })({color:'red'});}"
)]
#[case::nested_attrs("const Card=styled.div.attrs({title:alias({color:'red'})})({margin:1});")]
#[case::ordinary_attrs(
    "function f(styled){return styled.div({color:'blue'}).attrs({id:'x'});}const Card=styled.div({color:'red'});"
)]
#[case::generic_type(
    "import * as ui from '@devup-ui/react';function f(){const rules={color:'red'};const Card=ui.styled.div(function rules<rules>(p:rules){return {color:'blue'};});return [alias(rules),Card];}"
)]
#[case::parameter("function f(styled){return styled.div(p=>({color:p.color})).attrs({id:'x'});}")]
#[case::block("{const styled=ordinary;styled.div(p=>({color:p.color})).attrs({id:'x'});}")]
#[case::caught(
    "try{throw ordinary;}catch(styled){styled.div(p=>({color:p.color})).attrs({id:'x'});}"
)]
#[case::iteration(
    "for(const styled of factories){styled.div(p=>({color:p.color})).attrs({id:'x'});}"
)]
#[serial]
fn semantic_controls_compile_when_callback_or_factory_bindings_are_distinct(
    #[case] body: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!("import {{css,styled}} from '@devup-ui/react';const alias=css;{body}");
    // When
    let result = extract("intersection.tsx", &source, options(single_css));
    // Then
    assert!(result.is_ok(), "{result:?}");
}

#[rstest]
#[case::opaque("cx(getClass())")]
#[case::member("cx(source.getClass())")]
#[case::mixed("cx([1,on?getClass():paint({color:'red'})])")]
#[serial]
fn class_names_refuses_opaque_calls_when_callback_apis_are_renamed(
    #[case] expression: &str,
    #[values(false, true)] single_css: bool,
) {
    // Given
    reset_class_map();
    reset_file_map();
    let source = format!(
        "import {{ClassNames}} from '@emotion/react';export const App=()=> <ClassNames>{{({{css:paint,cx}})=><div className={{{expression}}}/ >}}</ClassNames>;"
    );
    // When
    let result = extract("intersection.tsx", &source, options(single_css));
    // Then
    let error = rejection(result);
    assert!(error.contains("intersection.tsx:1:"), "{error}");
    assert!(
        error.contains("cannot use") || error.contains("Cannot compose"),
        "{error}"
    );
}

#[rstest]
#[serial]
fn local_records_keep_fallback_units_when_native_and_emotion_consumers_mix(
    #[values(false, true)] single_css: bool,
) {
    // Given
    reset_class_map();
    reset_file_map();
    let source = "import {css as native} from '@devup-ui/react';import {css as paint} from '@emotion/react';export function f(){const rules={padding:[2,4],opacity:[0,0.5],margin:0,vars:{'--gap':7}};return [paint(rules),native({margin:2})];}";
    // When
    let output = extract("intersection.tsx", source, options(single_css))
        .unwrap_or_else(|error| panic!("{error}"));
    // Then
    let declarations: Vec<_> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) => Some((style.property(), style.value())),
            _ => None,
        })
        .collect();
    for expected in [
        ("padding", "2px;padding:4px"),
        ("opacity", "0;opacity:.5"),
        ("margin", "0"),
        ("margin", "8px"),
        ("--gap", "7"),
    ] {
        assert!(
            declarations.contains(&expected),
            "{expected:?}: {declarations:?}"
        );
    }
}

#[rstest]
#[case::safe(false)]
#[case::observed(true)]
#[serial]
fn imported_metadata_is_readable_only_when_local_rules_are_style_only(
    #[case] observed: bool,
    #[values(false, true)] single_css: bool,
) {
    // Given
    reset_class_map();
    reset_file_map();
    let allocator = Allocator::default();
    let source = "import {styled} from '@devup-ui/react';import {Card} from './base';export const Child=styled(Card)({ margin: 1 });";
    let program = parse(&allocator, "/src/app.tsx", source);
    let option = options(single_css);
    let base = format!(
        "import {{css,styled}} from '@devup-ui/react';const rules={{color:'pink'}};const seed=css(rules);export const Card=styled.div({{color:'red'}});export function f(){{const rules={{color:'blue'}};{}return css(rules);}}",
        if observed { "observe(rules);" } else { "" }
    );
    let resolver = move |specifier: &str, _: &str| {
        (specifier == "./base").then(|| crate::ResolvedModule {
            path: "/src/base.tsx".to_string(),
            code: base.clone(),
        })
    };
    // When
    let reading = crate::imported_styled::read(
        &oxc_ast::builder::AstBuilder::new(&allocator),
        &program,
        "/src/app.tsx",
        &option,
        Some(&resolver),
        crate::css_prop::CssProp::Off,
    );
    // Then
    let card = reading
        .components
        .get("Card")
        .unwrap_or_else(|| panic!("missing imported component metadata"));
    assert_eq!(card.definition.is_some(), !observed);
}

#[rstest]
#[serial]
fn unfinished_creator_is_refused_when_only_its_first_call_is_written(
    #[values(false, true)] single_css: bool,
) {
    // Given
    reset_class_map();
    reset_file_map();
    let source = "import {styled} from '@devup-ui/react';const creator=styled('section');";
    // When
    let result = extract("intersection.tsx", source, options(single_css));
    // Then
    let error = rejection(result);
    assert!(error.starts_with("intersection.tsx:1:"), "{error}");
    assert!(error.contains("`styled` is read at runtime"), "{error}");
}
