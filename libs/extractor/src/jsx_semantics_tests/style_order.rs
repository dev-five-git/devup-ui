use super::*;
use serial_test::serial;

#[test]
#[serial]
fn style_order_the_build_knows_is_kept() {
    let rendered = code(&format!(
        "{BOX}const ORDER = 3;\nexport const a = <Box styleOrder={{ORDER}} color=\"red\" />;\nexport const b = <Box styleOrder={{5 as number}} color=\"red\" />;\nexport const e = <Box styleOrder={{cond && 2}} color=\"red\" />;\nexport const f = <Box styleOrder=\"7\" color=\"red\" />;"
    ));

    for order in ["--3-", "--5-", "--255-", "--2-", "--7-"] {
        assert!(rendered.contains(order), "{order}: {rendered}");
    }
}

#[test]
#[serial]
fn explicit_absent_orders_when_written_as_c_d_h_report_the_authored_branches() {
    for (name, order, branches) in [
        ("c", "undefined", vec!["undefined"]),
        ("d", "cond ? 1 : undefined", vec!["undefined"]),
        ("h", "cond ? null : void 0", vec!["null", "void 0"]),
    ] {
        let source =
            format!("{BOX}export const {name} = <Box styleOrder={{{order}}} color=\"red\" />;");
        let message = error(&source);
        assert_eq!(message.lines().count(), branches.len(), "{message}");
        for branch in branches {
            let offset = source
                .find(branch)
                .unwrap_or_else(|| panic!("fixture branch"));
            let location = crate::locate("a.tsx", &source, offset);
            assert!(
                message
                    .lines()
                    .any(|line| line.starts_with(&format!("{location}:"))
                        && line.contains(&crate::style_order::invalid_order(branch))),
                "{message}"
            );
        }
    }
}

#[test]
#[serial]
fn empty_style_order_expression_is_a_located_parse_error() {
    let message = error(&format!(
        "{BOX}export const g = <Box styleOrder={{}} color=\"red\" />;"
    ));
    assert!(message.starts_with("a.tsx:2:34:"), "{message}");
    assert!(
        message.contains("JSX attributes must only be assigned a non-empty 'expression'"),
        "{message}"
    );
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
fn static_invalid_orders_when_written_as_strings_or_jsx_keep_main_diagnostics() {
    for (order, written) in [
        ("\"abc\"", "\"abc\""),
        ("<div />", "<div />"),
        ("<></>", "<></>"),
    ] {
        let source = format!("{BOX}export const a = <Box styleOrder={order} color=\"red\" />;");
        let message = error(&source);
        let offset = source
            .find(order)
            .unwrap_or_else(|| panic!("fixture order"));
        assert!(
            message.starts_with(&format!("{}:", crate::locate("a.tsx", &source, offset))),
            "{message}"
        );
        assert!(
            message.contains(&crate::style_order::invalid_order(written)),
            "{message}"
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
