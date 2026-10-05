use rstest::rstest;
use serial_test::serial;

use super::{css, extract, names, reset};

#[rstest]
#[case("positionTry", "{top:1,width:2,top:3}", "{width:2,top:3}")]
#[case(
    "viewTransitionClass",
    "{old:{opacity:0,width:2,opacity:1}}",
    "{old:{opacity:1,width:2}}"
)]
#[case(
    "viewTransitionClass",
    "{old:{opacity:0},new:{},old:{opacity:1}}",
    "{old:{opacity:1},new:{}}"
)]
#[serial]
fn duplicate_assignments_share_when_the_effective_content_matches(
    #[case] api: &str,
    #[case] duplicate: &str,
    #[case] effective: &str,
) {
    // Given
    reset();
    let source = format!("const a=stylex.{api}({duplicate});const b=stylex.{api}({effective});");
    // When
    let output = extract(&source);
    // Then
    let values = names(&output);
    assert_eq!(values[0], values[1]);
    assert_eq!(css(&output).len(), 1);
}

#[rstest]
#[serial]
fn position_keys_are_accepted_when_the_key_is_in_the_upstream_contract(
    #[values(
        "anchorName",
        "positionAnchor",
        "positionArea",
        "top",
        "right",
        "bottom",
        "left",
        "inset",
        "insetBlock",
        "insetBlockEnd",
        "insetBlockStart",
        "insetInline",
        "insetInlineEnd",
        "insetInlineStart",
        "margin",
        "marginBlock",
        "marginBlockEnd",
        "marginBlockStart",
        "marginInline",
        "marginInlineEnd",
        "marginInlineStart",
        "marginTop",
        "marginBottom",
        "marginLeft",
        "marginRight",
        "width",
        "height",
        "minWidth",
        "minHeight",
        "maxWidth",
        "maxHeight",
        "blockSize",
        "inlineSize",
        "minBlockSize",
        "minInlineSize",
        "maxBlockSize",
        "maxInlineSize",
        "alignSelf",
        "justifySelf",
        "placeSelf"
    )]
    key: &str,
) {
    // Given
    reset();
    let source = format!("const a=stylex.positionTry({{{key}:'auto'}});");
    // When
    let output = extract(&source);
    // Then
    assert_eq!(css(&output).len(), 1);
}
