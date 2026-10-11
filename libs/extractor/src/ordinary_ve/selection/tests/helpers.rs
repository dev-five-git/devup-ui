use super::super::plan::EscapeKind;
use super::{names, selected, text};

#[test]
fn includes_nested_selected_helpers_without_replaying_two_callers() {
    // Given
    let source = "import {createVar,style} from '@vanilla-extract/css';function make(space){function token(){return createVar()}const value=token();return style({margin:value,padding:space})}const first=make(8);const second=make(12);const host=window.document;";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(names(&plan.units), vec!["make", "first", "second"]);
    assert_eq!(plan.roots.len(), 2);
    assert_eq!(plan.roots[0].native_calls, plan.roots[1].native_calls);
    assert_eq!(plan.roots[0].native_calls.len(), 2);
    assert_eq!(
        plan.helper_calls
            .iter()
            .filter(|call| text(source, call.span).starts_with("make("))
            .count(),
        2
    );
    assert_eq!(names(&plan.consumed), vec!["make"]);
    assert_eq!(plan.escapes.len(), 0);
    assert_eq!(plan.checks.len(), 0);
}

#[test]
fn collects_unknown_branch_reads_without_rejecting_static_chosen_control_flow() {
    // Given
    let source = "import {createVar,style} from '@vanilla-extract/css';function make(enabled){if(enabled)return {token:createVar(),box:style({color:'blue'})};return {token:window.innerWidth,box:style({color:'red'})}}const value=make(true);const handler=()=>document.title;";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(names(&plan.units), vec!["make", "value"]);
    assert_eq!(plan.roots.len(), 1);
    assert_eq!(plan.escapes.len(), 0);
    let unknown = plan
        .reads
        .iter()
        .find(|read| read.name == "window")
        .unwrap_or_else(|| panic!("missing deferred window read"));
    assert_eq!(unknown.symbol, None);
    assert_eq!(text(source, unknown.span), "window");
    assert!(plan.reads.iter().all(|read| read.name != "document"));
}

#[test]
fn leaves_uncalled_private_native_helpers_out_of_the_execution_slice() {
    // Given
    let source = "import {style} from '@vanilla-extract/css';function unused(){return style({color:'red'})}const handler=()=>window.name;";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(plan.units.len(), 0);
    assert_eq!(plan.roots.len(), 0);
    assert_eq!(plan.native_calls.len(), 0);
    assert_eq!(names(&plan.consumed), vec!["unused"]);
    assert_eq!(plan.escapes.len(), 0);
}

#[test]
fn removes_an_uncalled_private_factory_without_eagerly_running_its_returned_native_closure() {
    // Given
    let source = "import {createVar} from '@vanilla-extract/css';function unused(){return ()=>createVar()}const handler=()=>window.name;";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(plan.units.len(), 0);
    assert_eq!(plan.roots.len(), 0);
    assert_eq!(plan.native_calls.len(), 0);
    assert_eq!(names(&plan.consumed), vec!["unused"]);
    assert_eq!(plan.escapes.len(), 0);
}

#[test]
fn removes_uncalled_private_unknown_native_branches_without_evaluating_their_keys() {
    // Given
    let source = "import * as ve from '@vanilla-extract/css';function unused(){return ve[window.name]({})}const handler=()=>document.title;";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(plan.roots.len(), 0);
    assert_eq!(names(&plan.consumed), vec!["unused"]);
    assert_eq!(plan.escapes.len(), 0);
    assert_eq!(plan.checks.len(), 0);
}

#[test]
fn selects_native_parameter_defaults_and_callback_bodies_only_through_callers() {
    // Given
    let source = "import {styleVariants,createVar,style} from '@vanilla-extract/css';function make(space=createVar()){return style({margin:space})}const variants=styleVariants({one:1,two:2},()=>make());const ignored=()=>make();";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(names(&plan.units), vec!["make", "variants"]);
    assert_eq!(plan.roots.len(), 1);
    assert_eq!(plan.roots[0].native_calls.len(), 3);
    assert_eq!(names(&plan.consumed), vec!["make", "ignored"]);
    assert_eq!(plan.escapes.len(), 0);
}

#[test]
fn reports_runtime_helper_reachability_at_the_original_reference() {
    // Given
    let source = "import {style} from '@vanilla-extract/css';function make(){return style({color:'red'})}const box=make();export const render=()=>make();";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(plan.roots.len(), 1);
    assert!(
        plan.escapes
            .iter()
            .any(|escape| escape.kind == EscapeKind::RuntimeHelper
                && text(source, escape.span) == "make")
    );
    assert!(
        plan.escapes
            .iter()
            .all(|escape| !escape.cause().is_empty() && !escape.fix().is_empty())
    );
}

#[test]
fn retains_legal_recursive_helper_dependencies_as_syntax_not_eager_execution() {
    // Given
    let source = "import {createVar} from '@vanilla-extract/css';function first(n){return n?second(n-1):createVar()}function second(n){return first(n)}const token=first(2);";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(names(&plan.units), vec!["first", "second", "token"]);
    assert_eq!(plan.roots.len(), 1);
    assert_eq!(plan.roots[0].native_calls.len(), 1);
    assert_eq!(plan.escapes.len(), 0);
}

#[test]
fn ignores_uncalled_nested_helper_inputs_when_an_iife_owns_native_creation() {
    // Given
    let source = "import {createVar} from '@vanilla-extract/css';const host=window.document;const token=(()=>{function unused(){return host}return createVar()})();";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(names(&plan.units), vec!["token"]);
    assert_eq!(plan.roots.len(), 1);
    assert_eq!(plan.roots[0].captures[0].name, "token");
    assert!(plan.reads.iter().all(|read| read.name != "host"));
}

#[test]
fn resolves_static_local_object_helpers_when_an_initializer_calls_a_method() {
    // Given
    let source = "import {createVar} from '@vanilla-extract/css';const helpers={nested:{make:()=>createVar()}};const token=helpers.nested.make();";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(names(&plan.units), vec!["helpers", "token"]);
    assert_eq!(plan.roots.len(), 1);
    assert_eq!(plan.roots[0].native_calls.len(), 1);
    assert_eq!(plan.escapes.len(), 0);
}

#[rstest::rstest]
#[case("const {make}=helpers;")]
#[case("const make=true?helpers.make:helpers.other;")]
fn follows_callable_destructuring_and_conditional_aliases_without_evaluating_a_branch(
    #[case] declaration: &str,
) {
    // Given
    let source = format!(
        "import {{createVar}} from '@vanilla-extract/css';const helpers={{make:()=>createVar(),other:()=>0}};{declaration}const token=make();"
    );
    // When
    let plan = selected(&source);
    // Then
    assert_eq!(names(&plan.units), vec!["helpers", "make", "token"]);
    assert_eq!(plan.roots.len(), 1);
    assert_eq!(plan.roots[0].native_calls.len(), 1);
    assert_eq!(plan.escapes.len(), 0);
}

#[test]
fn includes_pure_helper_captures_when_a_selected_data_initializer_calls_the_helper() {
    // Given
    let source = "import {style} from '@vanilla-extract/css';const size=8,host=window.name;function data(){return {padding:size}}const rules=data();const box=style(rules);";
    // When
    let plan = selected(source);
    // Then
    assert_eq!(names(&plan.units), vec!["size", "data", "rules", "box"]);
    assert_eq!(plan.roots.len(), 1);
    assert_eq!(plan.escapes.len(), 0);
}

#[rstest::rstest]
#[case("function factory(){return ()=>createVar()}const make=factory();")]
#[case("const factory=()=>()=>createVar();const make=factory();")]
#[case("function helper(){return createVar()}const make=helper.bind(null);")]
fn follows_known_returned_or_bound_helpers_without_replaying_the_factory(
    #[case] declaration: &str,
) {
    // Given
    let source = format!(
        "import {{createVar}} from '@vanilla-extract/css';{declaration}const token=make();"
    );
    // When
    let plan = selected(&source);
    // Then
    assert_eq!(plan.roots.len(), 1);
    assert_eq!(plan.roots[0].captures[0].name, "token");
    assert_eq!(plan.roots[0].native_calls.len(), 1);
    assert_eq!(plan.escapes.len(), 0);
}
