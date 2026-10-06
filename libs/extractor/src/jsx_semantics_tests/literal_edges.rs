use super::*;
use serial_test::serial;

#[test]
#[serial]
fn literal_order_when_comment_trivia_surrounds_exact_hole_preserves_number_type() {
    // Given: trivia belongs to CSS, not the interpolated primitive.
    let source = "import {css} from '@devup-ui/react'; const a=css`color:red;style-order:/*before*/${+'02'}/*after*/`;";
    // When: the literal is compiled.
    let result = output(source);
    // Then: the exact Number is order 2, rather than a noncanonical string.
    assert!(result.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property() == "color" && style.style_order() == Some(2))));
}

#[test]
#[serial]
fn literal_order_when_comment_contains_interpolation_keeps_source_effects() {
    // Given: tag execution evaluates holes even in CSS comments.
    let source = "import {css} from '@devup-ui/react'; const mark=(name)=>(trace.push(name),true); const a=css`/*${mark('before')}*/color:${mark('color')?'red':'blue'};style-order:${mark('order')?2:3};/*${mark('after')}*/`;";
    // When: compiled source executes.
    let result = whole::evaluate(source, "a");
    // Then: discarded CSS still evaluates its original holes in source order.
    assert_eq!(
        result.trace,
        serde_json::json!(["before", "color", "order", "after"])
    );
}

#[test]
#[serial]
fn literal_order_when_direct_styled_text_captures_construction_selection() {
    // Given: direct call text is an accepted styled rule surface.
    let source = "import {styled} from '@devup-ui/react'; const __devupForwardRef=(render)=>render; let active=true; const config={get active(){trace.push('construct');return active}}; const Card=styled('div',`style-order:${config.active?2:3};color:red`); active=false; const a=Card({},null); const b=Card({},null);";
    // When: construction and two renders execute.
    let result = whole::evaluate(source, "[a.props.className,b.props.className]");
    // Then: the saved construction value remains order 2.
    assert_eq!(result.trace, serde_json::json!(["construct"]));
    assert_eq!(result.element[0], result.element[1]);
    assert!(
        result.element[0]
            .as_str()
            .required("constructed styled result must have classes")
            .contains("--2-")
    );
}

#[test]
#[serial]
fn literal_order_when_stylex_value_contains_metadata_text_preserves_value() {
    // Given: StyleX has declaration values, not a public CSS-text rule surface.
    let source = "import * as stylex from '@stylexjs/stylex'; const styles=stylex.create({base:{content:'style-order:2'}});";
    // When: the declaration value is compiled.
    let result = output(source);
    // Then: ordinary content is not treated as a metadata scope.
    assert!(result.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.property() == "content" && style.value() == "style-order:2")));
}

#[test]
#[serial]
fn literal_order_when_descriptor_value_contains_metadata_text_preserves_value() {
    // Given: descriptor strings are values, unlike descriptor template entries.
    let source = "import {globalCss} from '@devup-ui/react'; globalCss({fontFaces:[{fontFamily:'style-order:2',src:'font.woff'}]});";
    // When: the descriptor object is compiled.
    let result = output(source);
    // Then: the descriptor value remains ordinary text.
    assert!(result.styles.iter().any(|style| matches!(style, ExtractStyleValue::FontFace(style) if style.properties.iter().any(|(key,value)| key == "font-family" && value == "style-order:2"))));
}

#[test]
#[serial]
fn literal_order_when_frame_value_contains_metadata_text_preserves_value() {
    // Given: a frame's ordinary declaration is not another CSS scope.
    let source = "import {keyframes} from '@devup-ui/react'; const a=keyframes({from:{content:'style-order:2'}});";
    // When: the frame object is compiled.
    let result = output(source);
    // Then: it retains content rather than rejecting an unrelated string.
    assert!(result.styles.iter().any(|style| matches!(style, ExtractStyleValue::Keyframes(style) if style.keyframes.values().flatten().any(|style| style.property() == "content" && style.value() == "style-order:2"))));
}

#[test]
#[serial]
fn literal_order_when_frame_responsive_values_contain_text_preserves_values() {
    let result = output(
        "import {keyframes} from '@devup-ui/react'; const a=keyframes({from:{content:['style-order:2']}});",
    );
    assert!(result.styles.iter().any(|style| matches!(style, ExtractStyleValue::Keyframes(style) if style.keyframes.values().flatten().any(|style| style.value() == "style-order:2"))));
}

#[test]
#[serial]
fn literal_order_when_callback_returns_text_retains_lazy_return_effects() {
    let source = "import {styled} from '@devup-ui/react'; const __devupForwardRef=(render)=>render; const Card=styled.div(p=>{trace.push(p.active);if(p.active)return `style-order:${(trace.push('yes'),true)?2:3};color:red`;return 'style-order:4;color:blue'}); const a=Card({active:true},null); const b=Card({active:false},null);";
    let result = whole::evaluate(source, "[a.props.className,b.props.className]");
    assert_eq!(result.trace, serde_json::json!([true, "yes", false]));
    assert!(
        result.element[0]
            .as_str()
            .required("active text callback must emit classes")
            .contains("--2-")
    );
    assert!(
        result.element[1]
            .as_str()
            .required("inactive text callback must emit classes")
            .contains("--4-")
    );
}

#[test]
#[serial]
fn literal_order_when_no_effect_key_follows_crlf_locates_original_key() {
    let source = "import {keyframes} from '@devup-ui/react';\r\nconst a=keyframes`/* 한글 */\r\nfrom {\r\n style-order:2;opacity:0}`;";
    let message = error(source);
    assert!(message.starts_with("a.tsx:4:2:"), "{message}");
}

#[test]
#[serial]
fn literal_order_when_number_hole_has_trivia_rejects_noncanonical_string() {
    let source =
        "import {css} from '@devup-ui/react'; const a=css`style-order:${'02'}/*after*/;color:red`;";
    let message = error(source);
    assert!(message.contains("canonical decimal string"), "{message}");
}
