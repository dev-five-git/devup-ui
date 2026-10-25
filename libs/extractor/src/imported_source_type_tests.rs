use super::*;

#[rstest::rstest]
#[case("base.mdown")]
#[case("base.ts")]
#[serial_test::serial]
fn compiled_styled_computation_when_composed_keeps_mode_and_emotion_units(
    #[case] path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given: compiled JSX precedes a computed Emotion rule in a non-JSX filename.
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let resolver = |specifier: &str, _: &str| {
        match specifier {
        "./ignored" => Some(ModuleResolution::Ignored),
        "./base" => Some(ModuleResolution::Resolved(crate::ResolvedModule {
            path: path.into(),
            code: "import styled from '@emotion/styled'; import empty from './ignored'; export const view=<div/>; const scaled=n=>n*4; export const Base=styled.div({...empty,padding:scaled(2),color:'red'});".into(),
            source_type: Some(crate::ExtractSourceType::CompiledMdx),
        })),
        _ => None,
    }
    };
    let option = ExtractOption {
        import_aliases: std::collections::HashMap::from([(
            "@emotion/styled".into(),
            crate::ImportAlias::DefaultToNamed("styled".into()),
        )]),
        ..ExtractOption::default()
    };
    // When: the imported definition requires build-time evaluation and a retry.
    let output = crate::extract_with_modules(
        "page.tsx",
        "import styled from '@emotion/styled'; import {Base} from './base'; export const Child=styled(Base)({color:'blue'});",
        option,
        false,
        &resolver,
    )?;
    // Then: later rules win, ignored spreads stay empty, and computed numbers stay px.
    let styles: BTreeSet<_> = output
        .styles
        .iter()
        .map(|style| match style {
            ExtractStyleValue::Static(style) => (style.property.as_str(), style.value.as_str()),
            other => panic!("expected static rule, got {other:?}"),
        })
        .collect();
    assert_eq!(
        styles,
        BTreeSet::from([("color", "blue"), ("padding", "8px")])
    );
    assert_eq!(output.dependencies, [path]);
    assert!(output.code.contains("DevupAs = \"div\""), "{}", output.code);
    Ok(())
}

#[test]
#[serial_test::serial]
fn compiled_callback_when_importing_constants_preserves_local_capture_and_px()
-> Result<(), Box<dyn std::error::Error>> {
    // Given: a bound callback captures an outer parameter beside a compiled child's constant rule.
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let resolver = |_: &str, _: &str| {
        Some(ModuleResolution::Resolved(crate::ResolvedModule {
            path: "tokens.ts".into(),
            code: "export const view=<div/>; export const size=8;".into(),
            source_type: Some(crate::ExtractSourceType::CompiledMdx),
        }))
    };
    let option = ExtractOption {
        import_aliases: std::collections::HashMap::from([(
            "@emotion/styled".into(),
            crate::ImportAlias::DefaultToNamed("styled".into()),
        )]),
        ..ExtractOption::default()
    };
    // When: an inner parameter shadows the capture at a styled rule use.
    let output = crate::extract_with_source_type(
        "page.mdown",
        "import styled from '@emotion/styled'; import {size} from './tokens'; export const view=<div/>; export function outer(tone) { const rules=p=>({ color: tone }); return function inner(tone) { return styled.div(rules,{ padding: size }); }; }",
        option,
        false,
        Some(&resolver),
        Some(crate::ExtractSourceType::CompiledMdx),
    )?;
    // Then: semantic capture recovery coexists with source-mode-aware constant inlining.
    assert!(
        output.code.contains("function outer(__devup_tone"),
        "{}",
        output.code
    );
    assert!(
        output.code.contains("function inner(tone)"),
        "{}",
        output.code
    );
    assert!(
        output.styles.iter().any(|style| matches!(style,
            ExtractStyleValue::Static(style) if style.property == "padding" && style.value == "8px"
        )),
        "{output:?}"
    );
    assert!(
        output.styles.iter().any(|style| matches!(style,
            ExtractStyleValue::Dynamic(style) if style.property() == "color"
        )),
        "{output:?}"
    );
    assert_eq!(output.dependencies, ["tokens.ts"]);
    Ok(())
}

#[test]
fn source_type_returns_no_constants_when_an_unvalidated_internal_path_is_unknown() {
    let option = ExtractOption::default();
    let mut modules = Modules {
        resolver: None,
        option: &option,
        exports: FxHashMap::default(),
        loading: Vec::new(),
    };
    assert_eq!(
        modules
            .read("token.custom", "export const PRIMARY = 'red';", None)
            .len(),
        0
    );
}

#[test]
#[serial_test::serial]
fn ignored_reexports_remain_known_when_compiled_child_rules_are_composed() {
    // Given: a compiled child reads ignored named/default exports inside nested rules.
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let resolver = |specifier: &str, _: &str| {
        match specifier {
        "./ignored" => Some(ModuleResolution::Ignored),
        "./bridge" => Some(ModuleResolution::Resolved(crate::ResolvedModule {
            path: "bridge.js".into(),
            code: "export {missing as absent, default as empty} from './ignored';".into(),
            source_type: None,
        })),
        "./base" => Some(ModuleResolution::Resolved(crate::ResolvedModule {
            path: "base.ts".into(),
            code: "import {css} from '@devup-ui/react'; import {absent,empty} from './bridge'; const undefined='wrong'; export const view=<div/>; export const base=css({color:'red',width:[absent,'17px'],_focus:empty,_hover:{...empty,color:'blue'}});".into(),
            source_type: Some(crate::ExtractSourceType::CompiledMdx),
        })),
        _ => None,
    }
    };
    // When: a selected native chain composes that child with a later override.
    let output = crate::extract_with_modules(
        "page.ts", "import {css} from '@devup-ui/react'; import {base} from './base'; export const cls=css(base,{color:'green'});",
        ExtractOption::default(), false, &resolver,
    ).unwrap_or_else(|error| panic!("{error}"));
    // Then: ignored values are absent, the child grammar is honored, and later rules win.
    for (property, value) in [("color", "green"), ("color", "blue"), ("width", "17px")] {
        assert!(output.styles.iter().any(|style| matches!(style,
            ExtractStyleValue::Static(style) if style.property == property && style.value == value
        )), "{property}:{value}");
    }
    assert!(!output.styles.iter().any(|style| matches!(style,
        ExtractStyleValue::Static(style) if style.value == "wrong" || style.value == "red"
    )));
}
