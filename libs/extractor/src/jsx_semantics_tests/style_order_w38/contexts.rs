use super::*;

#[test]
#[serial]
fn global_orders_when_root_body_and_nested_objects_set_them_are_static_layers() {
    let actual = orders(
        "import {globalCss} from '@devup-ui/react'; globalCss({styleOrder:2,body:{color:'red',_hover:{styleOrder:3,color:'blue'}},'@media print':{styleOrder:4,p:{color:'green'}}});",
    );
    assert_eq!(
        actual,
        vec![
            ("color".to_string(), 0, Some(2)),
            ("color".to_string(), 0, Some(3)),
            ("color".to_string(), 0, Some(4))
        ]
    );
}

#[test]
#[serial]
fn global_order_when_conditional_is_a_located_metadata_error() {
    let message = error(
        "import {globalCss} from '@devup-ui/react'; const f=(on)=>globalCss({body:{styleOrder:on?2:3,color:'red'}});",
    );
    assert!(
        message.starts_with("a.tsx:1:") && message.contains("styleOrder"),
        "{message}"
    );
}

#[test]
#[serial]
fn font_face_order_when_written_as_a_descriptor_is_a_located_no_effect_error() {
    for order in ["2", "on ? 1 : 2", "0"] {
        let message = error(&format!(
            "import {{globalCss}} from '@devup-ui/react'; const a=(on)=>globalCss({{fontFaces:[{{fontFamily:'Example',src:'url(example.woff2)',styleOrder:{order}}}]}});"
        ));
        assert!(
            message.starts_with("a.tsx:1:")
                && message.contains("styleOrder")
                && message.contains("no effect in fontFaces descriptors"),
            "{message}"
        );
    }
}

#[test]
#[serial]
fn keyframe_order_when_written_at_any_depth_is_a_located_no_effect_error() {
    for rules in [
        concat!("{", "styleOrder:2,from:{opacity:0}}"),
        "{from:{styleOrder:2,opacity:0}}",
        "{from:{_hover:{styleOrder:2,opacity:0}}}",
    ] {
        let message = error(&format!(
            "import {{keyframes}} from '@devup-ui/react'; const a=keyframes({rules});"
        ));
        assert!(
            message.starts_with("a.tsx:1:")
                && message.contains("styleOrder")
                && message.contains("no effect"),
            "{message}"
        );
    }
}

#[test]
#[serial]
fn stylex_order_when_used_as_a_declaration_is_rejected_but_names_are_allowed() {
    for call in [
        "create({a:{styleOrder:2}})",
        concat!("create({a:(x)=>({", "styleOrder:x})})"),
        concat!("create({a:{':hover':{", "styleOrder:2}}})"),
        concat!("positionTry({", "styleOrder:2})"),
        concat!("viewTransitionClass({", "styleOrder:2})"),
        "keyframes({from:{styleOrder:2}})",
    ] {
        let message = error(&format!(
            "import * as stylex from '@stylexjs/stylex'; const a=stylex.{call};"
        ));
        assert!(
            message.starts_with("a.tsx:1:")
                && message.contains("styleOrder")
                && message.contains("no effect"),
            "{call}: {message}"
        );
    }
    assert!(compile("import * as stylex from '@stylexjs/stylex'; const vars=stylex.defineVars({styleOrder:'red'}); const styles=stylex.create({styleOrder:{color:'red'}});").is_ok());
}

#[test]
#[serial]
fn invalid_branch_when_written_in_css_reports_the_value_location() {
    let message = error(
        "import {css} from '@devup-ui/react';\nconst a=(on)=>css({styleOrder:on?2:255,color:'red'});",
    );
    assert!(message.starts_with("a.tsx:2:36:"), "{message}");
    assert!(message.contains("styleOrder"));
}

#[test]
#[serial]
fn records_and_passthrough_data_when_named_style_order_are_not_metadata() {
    let source = concat!(
        "import {css} from '@devup-ui/react'; const a=css({color:'red',props:{",
        "styleOrder:999},styleVars:{",
        "styleOrder:999},selectors:{styleOrder:{color:'blue'}},'@layer':{styleOrder:{color:'green'}}});"
    );
    assert!(compile(source).is_ok());
}

#[test]
#[serial]
fn invalid_metadata_when_consumed_without_css_validation_is_forwarded_once() {
    for body in [
        "const a=(on)=><Box _hover={{styleOrder:on?2:255,color:'red'}}/>;",
        "const a=(on)=>jsx(Box,{_hover:{styleOrder:on?2:255,color:'red'}});",
        "const a=(on)=><div css={{styleOrder:on?2:255,color:'red'}}/>;",
        "const a=(on)=><Global styles={{body:{styleOrder:on?2:255,color:'red'}}}/>;",
        "const a=<Global styles={{fontFaces:[{fontFamily:'A',src:'url(a)',styleOrder:2}]}}/>;",
    ] {
        let source =
            format!("{EMOTION}{BOX}{JSX_RUNTIME}import {{Global}} from '@emotion/react';{body}");
        let message = compile_emotion(&source)
            .required_err("invalid metadata must be forwarded as a located diagnostic");
        assert!(message.contains("styleOrder"), "{body}: {message}");
        assert_eq!(message.matches("cannot use").count(), 1, "{message}");
    }
}

#[test]
fn template_order_test_when_nonempty_uses_string_truthiness() {
    use oxc_ast::ast::Statement;
    for text in ["0", "abc"] {
        let allocator = Allocator::default();
        let source = format!("`{text}` ? 2 : 3;");
        let parsed = Parser::new(&allocator, &source, SourceType::ts()).parse();
        let Statement::ExpressionStatement(statement) = &parsed.program.body[0] else {
            panic!("expression")
        };
        assert!(matches!(
            crate::style_order::parse(&statement.expression, &allocator),
            Ok(crate::style_order::Order::Static(2))
        ));
    }
}
