use super::w38o_logical_oracle::{Expected, verify};
use super::w38o_logical_source::{Fixture, observe};
use super::*;
use rstest::rstest;
use serial_test::serial;

#[test]
#[serial]
fn literal_control_when_order_is_omitted_stays_outside_ordered_layers() {
    // Given: the orderless literal sibling has no guard or saved-result supplier.
    let input = Fixture {
        outer_producer: "",
        returned_expression: "css({color:'red'})",
    };
    // When: public extraction and exactly one generated render execute.
    let actual = observe(&input, "render(false,false,false)");
    // Then: the default class and complete unlayered stylesheet are preserved.
    verify(
        &actual,
        Expected {
            inventory: &["red"],
            selected: &["red"],
            trace: &[],
            order: None,
        },
    );
}

#[test]
#[serial]
fn coalesce_control_when_left_is_null_selects_the_ordered_fallback() {
    // Given: null differs from the empty string returned by an inactive css call.
    let input = Fixture {
        outer_producer: "",
        returned_expression: "css(null??{styleOrder:2,color:'blue'})",
    };
    // When: public extraction and exactly one generated render execute.
    let actual = observe(&input, "render(false,false,false)");
    // Then: the blue fallback supplies the entire ordered inventory and selection.
    verify(
        &actual,
        Expected {
            inventory: &["blue"],
            selected: &["blue"],
            trace: &[],
            order: Some(2),
        },
    );
}

#[rstest]
#[case::skipped(false, &[])]
#[case::selected(true, &["red"])]
#[serial]
fn guard_control_when_no_base_exists_preserves_absence_or_selection(
    #[case] outer: bool,
    #[case] selected: &[&str],
) {
    // Given: a single ordered rule has no base class to hide a false guard.
    let input = Fixture {
        outer_producer: "",
        returned_expression: "css(mark('outer',outer)&&{styleOrder:2,color:'red'})",
    };
    let invocation = format!("render(false,{outer},false)");
    // When: public extraction and exactly one generated render execute.
    let actual = observe(&input, &invocation);
    // Then: the guard runs once and false selects no declaration or class.
    verify(
        &actual,
        Expected {
            inventory: &["red"],
            selected,
            trace: &["outer"],
            order: Some(2),
        },
    );
}

#[test]
#[serial]
fn classnames_when_saved_producer_is_inside_multistatement_child_is_rejected() {
    // Given: the original L1 child declares saved before returning its selection.
    let source = "import {ClassNames} from '@emotion/react';\nconst render=(active,outer,inner)=>{const mark=(n,v)=>(trace.push(n),v);return <ClassNames>{({css})=>{const saved=css(mark('construct',active)?{styleOrder:2,color:'red'}:null);return css(saved||{styleOrder:2,color:'blue'});}}</ClassNames>};";
    let expected = "a.tsx:2:93: `<ClassNames>` cannot use `({ css }) => { const saved = css(mark(\"construct\", active) ? { styleOrder: 2, color: \"red\" } : null); return css(saved || { styleOrder: 2, color: \"blue\" }); }` at build time: it takes only a child function of `{ css, cx, theme }` giving what it renders at once";
    // When: the real public alias extractor handles the unsupported child.
    let actual = compile_emotion(source).required_err("multistatement child must reject");
    // Then: the complete original rejection and exact authored location remain.
    assert_eq!(actual, expected);
}
