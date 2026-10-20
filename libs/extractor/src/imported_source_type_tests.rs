use super::*;

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
