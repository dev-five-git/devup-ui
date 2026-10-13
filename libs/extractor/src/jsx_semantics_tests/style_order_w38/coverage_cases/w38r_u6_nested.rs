use super::inventory;
use super::*;
use rstest::rstest;

#[rstest]
#[case::nested_helper("cx('left',cx('right'))", "left right", &[])]
#[case::css_beside_class(
    "cx('left',css({color:'red'}))",
    "left color-0-red--255-a",
    &[("color", "red", 0, None)]
)]
#[serial]
fn w38r_u6_css_prop_when_it_composes_class_helpers_reads_them_before_they_are_compiled(
    #[case] expression: &str,
    #[case] classes: &str,
    #[case] declarations: &[(&str, &str, u8, Option<u8>)],
) {
    // Given: a css prop inside ClassNames still holds the uncompiled helper calls.
    let source = format!(
        "import {{ClassNames,jsx}} from '@emotion/react';const render=()=> <ClassNames>{{({{css,cx}})=>jsx('div',{{css:{expression}}})}}</ClassNames>;"
    );
    // When: the legacy preflight decomposes the call and its arguments.
    let actual = compile_emotion(&source).required("class helpers in a css prop compile");
    let evaluated = whole::evaluate_code(&actual.code, "render().props.className");
    // Then: helper arguments keep their literal order in one class string.
    assert_eq!(evaluated.element, serde_json::json!(classes));
    assert_eq!(evaluated.trace, serde_json::json!([]));
    inventory(&actual, declarations);
}

#[rstest]
#[case::chosen(true, "color-0-red--2-a ")]
#[case::other(false, " color-0-blue--3-a")]
#[serial]
fn w38r_u6_css_prop_when_choice_holds_ordered_tags_lowers_each_branch(
    #[case] flag: bool,
    #[case] classes: &str,
) {
    // Given: each branch of a css prop is an ordered css tag.
    let source = "import {jsx} from '@emotion/react';import {css} from '@devup-ui/react';const a=(f)=>jsx('div',{css:f?css`style-order:2;color:red;`:css`style-order:3;color:blue;`});";
    // When: the branches are visited as css prop values.
    let actual = compile_emotion(source).required("ordered tags in a choice compile");
    let evaluated = whole::evaluate_code(&actual.code, &format!("a({flag}).props.className"));
    // Then: only the chosen branch applies, with the order its own text states.
    assert_eq!(evaluated.element, serde_json::json!(classes));
    assert_eq!(evaluated.trace, serde_json::json!([]));
    inventory(
        &actual,
        &[("color", "red", 0, Some(2)), ("color", "blue", 0, Some(3))],
    );
}
