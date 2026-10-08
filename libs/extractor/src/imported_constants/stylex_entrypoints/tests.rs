use rstest::rstest;
use rustc_hash::FxHashMap;
use serial_test::serial;

use super::*;
use crate::ExtractOption;

fn exports(body: &str) -> FxHashMap<String, Constant> {
    let option = ExtractOption::default();
    let mut modules = Modules {
        resolver: None,
        option: &option,
        exports: FxHashMap::default(),
        loading: vec![],
    };
    modules.read(
        "/src/imported-transitions.ts",
        &format!("import * as sx from '@stylexjs/stylex';{body}"),
    )
}

#[rstest]
#[case(
    "positionTry",
    "{top:null,right:2,bottom:false,left:undefined}",
    "{right:2}"
)]
#[case(
    "viewTransitionClass",
    "{group:{opacity:null},old:{opacity:false},new:{opacity:undefined}}",
    "{group:{},old:{},new:{}}"
)]
#[serial]
fn imported_rule_identity_omits_known_empty_values_when_constants_are_evaluated(
    #[case] api: &str,
    #[case] input: &str,
    #[case] clean: &str,
) {
    // Given
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let source =
        format!("export const result=sx.{api}({input});export const clean=sx.{api}({clean});");
    // When
    let values = exports(&source);
    // Then
    let Some(Constant::String(expected)) = values.get("clean") else {
        panic!("missing clean identity: {values:?}");
    };
    assert!(matches!(values.get("result"), Some(Constant::String(actual)) if actual == expected));
}

#[rstest]
#[case("positionTry(true)")]
#[case("positionTry([1])")]
#[case(concat!("positionTry(", "{", "top:true", "}", ")"))]
#[case("viewTransitionClass({group:{opacity:true}})")]
#[case("keyframes('not frames')")]
#[case(concat!("keyframes(", "{", "to:1", "}", ")"))]
#[serial]
fn invalid_imported_rules_publish_no_identity_when_the_constant_shape_is_rejected(
    #[case] call: &str,
) {
    // Given
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    let source = format!("export const result=sx.{call};export const control='kept';");
    // When
    let values = exports(&source);
    // Then
    assert!(!values.contains_key("result"), "{values:?}");
    assert!(matches!(values.get("control"), Some(Constant::String(value)) if value == "kept"));
}

#[rstest]
#[case(StylexFunction::Create)]
#[case(StylexFunction::Props)]
#[case(StylexFunction::Attrs)]
#[case(StylexFunction::DefineVars)]
#[case(StylexFunction::CreateTheme)]
#[case(StylexFunction::CreateThemeContract)]
#[case(StylexFunction::DefineConsts)]
#[case(StylexFunction::FirstThatWorks)]
#[case(StylexFunction::Include)]
#[case(StylexFunction::Types)]
fn standalone_rule_evaluator_rejects_other_apis_when_given_a_valid_rule_object(
    #[case] function: StylexFunction,
) {
    // Given
    let allocator = Allocator::default();
    let program = Parser::new(&allocator, "", SourceType::ts())
        .parse()
        .program;
    let expression = Parser::new(&allocator, "({top:1})", SourceType::ts())
        .parse_expression()
        .unwrap_or_else(|error| panic!("{error:?}"));
    let mut scope = ModuleScope::new("/src/rules.ts", &program, None);
    let option = ExtractOption::default();
    let mut modules = Modules {
        resolver: None,
        option: &option,
        exports: FxHashMap::default(),
        loading: vec![],
    };
    // When
    let result = scope.evaluate_stylex_rule(&mut modules, function, &expression);
    // Then
    assert!(result.is_none(), "{result:?}");
}
