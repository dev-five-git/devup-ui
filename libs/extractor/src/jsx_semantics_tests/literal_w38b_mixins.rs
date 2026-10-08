use super::*;
use serial_test::serial;

#[test]
#[serial]
fn mixin_when_conditional_selects_only_one_branch() {
    // Given: distinct mixin branches under one root order.
    let source = "import {css} from '@devup-ui/react';const a=(on)=>css`background:black;${on?{color:'red'}:{color:'blue'}};style-order:2`;";
    // When: the emitted function selects both alternatives in separate calls.
    let compiled = output(source);
    let actual = whole::evaluate_code(&compiled.code, "[a(true),a(false)]");
    // Then: black is common; red and blue never leak across alternatives.
    let yes = actual.element[0].as_str().required("true class");
    let no = actual.element[1].as_str().required("false class");
    assert!(
        yes.contains("red")
            && !yes.contains("blue")
            && yes.contains("black")
            && yes.contains("--2-"),
        "{yes}\n{}\n{:?}",
        compiled.code,
        compiled.styles
    );
    assert!(
        no.contains("blue") && !no.contains("red") && no.contains("black") && no.contains("--2-"),
        "{no}"
    );
}

#[test]
#[serial]
fn mixin_when_external_class_preserves_each_consumer() {
    // Given: an external class is opaque, not a static rule object.
    let source = concat!(
        "import {css} from '@devup-ui/react';",
        "const a=(external)=>css`color:red;${external};style-order:2`;"
    );
    // When: distinct external consumers invoke the compiled function.
    let actual = whole::evaluate(source, "[a('external-a'),a('external-b')]");
    // Then: each original class survives with ordered red.
    for (index, external) in ["external-a", "external-b"].iter().enumerate() {
        let class = actual.element[index].as_str().required("class");
        assert!(
            class.contains(external) && class.contains("red") && class.contains("--2-"),
            "{class}"
        );
    }
}

#[test]
#[serial]
fn mixin_when_class_expression_is_unreadable_locates_class() {
    // Given: a parser class expression cannot be read as styles.
    let source = "import {css} from '@devup-ui/react';\nconst a=css`color:red;${class Bad{}};style-order:2`;";
    let column = source
        .lines()
        .nth(1)
        .required("line")
        .find("class Bad")
        .required("class")
        + 1;
    // When: public extraction rejects the mixin.
    let actual = error(source);
    // Then: the authored class owns the diagnostic.
    assert!(
        actual.starts_with(&format!("a.tsx:2:{column}:")),
        "{actual}"
    );
}

#[test]
#[serial]
fn mixin_when_root_order_invalid_keeps_definitive_authored_error() {
    // Given: a saved valid base cannot excuse invalid root metadata.
    let source = "import {css} from '@devup-ui/react';\nconst base=css({color:'red'});const a=css`background:blue;${base};style-order:255`;";
    let column = source
        .lines()
        .nth(1)
        .required("line")
        .find("255")
        .required("order")
        + 1;
    // When: the public pipeline rejects metadata before fallback.
    let actual = error(source);
    // Then: the original invalid order remains located.
    assert!(
        actual.starts_with(&format!("a.tsx:2:{column}:")),
        "{actual}"
    );
}

#[test]
#[serial]
fn mixin_when_root_order_absent_keeps_base_and_nested_order_distinct() {
    // Given: only the nested hover has metadata.
    let source = "import {css} from '@devup-ui/react';const base=css({backgroundColor:'blue'});const a=css`${base};&:hover{style-order:3;color:red}`;";
    // When: public mixin composition lowers the ordered envelope.
    let actual = output(source);
    // Then: root blue is ordinary while hover red alone receives order 3.
    assert!(actual.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.value()=="blue" && style.style_order()!=Some(3))));
    assert!(actual.styles.iter().any(|style| matches!(style, ExtractStyleValue::Static(style) if style.value()=="red" && style.style_order()==Some(3) && style.selector().is_some())));
}
