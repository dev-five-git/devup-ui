use super::*;
use serial_test::serial;

#[test]
#[serial]
fn style_order_the_build_knows_is_kept() {
    let rendered = code(&format!(
        "{BOX}const ORDER = 3;\nexport const a = <Box styleOrder={{ORDER}} color=\"red\" />;\nexport const b = <Box styleOrder={{5 as number}} color=\"red\" />;\nexport const c = <Box styleOrder={{undefined}} color=\"red\" />;\nexport const d = <Box styleOrder={{cond ? 1 : undefined}} color=\"red\" />;\nexport const e = <Box styleOrder={{cond && 2}} color=\"red\" />;\nexport const f = <Box styleOrder=\"7\" color=\"red\" />;\nexport const g = <Box styleOrder={{}} color=\"red\" />;\nexport const h = <Box styleOrder={{cond ? null : void 0}} color=\"red\" />;"
    ));

    for order in ["--3-", "--5-", "--255-", "--1-", "--2-", "--7-"] {
        assert!(rendered.contains(order), "{order}: {rendered}");
    }
}

#[test]
#[serial]
fn a_runtime_style_order_is_a_located_error() {
    let message = error(&format!(
        "{BOX}export const a = (runtime) => <Box styleOrder={{runtime}} color=\"red\" />;"
    ));

    assert!(message.starts_with("a.tsx:2:"), "{message}");
    assert!(
        message.contains("`<Box>` cannot use `runtime`"),
        "{message}"
    );
}

#[test]
#[serial]
fn style_order_that_is_not_a_number_or_a_condition_of_numbers_is_an_error() {
    for (order, written) in [
        ("{cond || 2}", "cond || 2"),
        ("{cond ? 1 : other}", "cond ? 1 : other"),
        ("{cond ? other : 2}", "cond ? other : 2"),
        ("{cond && other}", "cond && other"),
        ("\"abc\"", "\"abc\""),
        ("<div />", "<element>"),
        ("<></>", "<>...</>"),
    ] {
        let message = error(&format!(
            "{BOX}export const a = (cond, other) => <Box styleOrder={order} color=\"red\" />;"
        ));

        assert!(
            message.contains(&format!("`<Box>` cannot use `{written}`")),
            "{order}: {message}"
        );
    }
}

#[test]
#[serial]
fn a_call_with_a_runtime_style_order_is_a_located_error() {
    for order in ["runtime", "cond ? 1 : runtime", "cond && runtime"] {
        let message = error(&format!(
            "{BOX}{JSX_RUNTIME}export const a = (cond, runtime) => jsx(Box, {{ styleOrder: {order}, color: 'red' }});"
        ));

        assert!(
            message.contains(&format!("cannot use `{order}`")),
            "{order}: {message}"
        );
    }
}

#[test]
#[serial]
fn a_call_with_a_known_style_order_compiles() {
    let rendered = code(&format!(
        "{BOX}{JSX_RUNTIME}export const a = (cond) => jsx(Box, {{ styleOrder: cond ? 2 : 4, color: 'red' }});\nexport const b = jsx(Box, {{ styleOrder: 6 as number, color: 'red' }});\nexport const c = jsx(Box, {{ color: 'red' }});"
    ));

    for order in ["--2-", "--4-", "--6-"] {
        assert!(rendered.contains(order), "{order}: {rendered}");
    }
}
