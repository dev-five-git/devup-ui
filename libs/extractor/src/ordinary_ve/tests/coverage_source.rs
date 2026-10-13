use serial_test::serial;

use super::demand_support::{TestResult, reset, run, static_value};

#[test]
#[serial]
fn native_var_slice_avoids_both_generated_name_collisions() -> TestResult {
    reset();
    let code = concat!(
        "import {style} from '@vanilla-extract/css';",
        "const __ve_api_style__=window.name,__ve_api_style__$=window.name,",
        "__ve_native_read__=window.name,__ve_native_read__$=window.name;",
        "var box=style({color:'purple',padding:13});throw new Error('host only');"
    );

    let output = run("/collisions.ts", code, &[])?;

    assert_eq!(static_value(&output, "color"), "purple");
    assert_eq!(static_value(&output, "padding"), "13px");
    assert!(output.code.contains("var box ="), "{}", output.code);
    assert!(
        output.code.contains("throw new Error(\"host only\")"),
        "{}",
        output.code
    );
    Ok(())
}

#[test]
#[serial]
fn consumer_view_copies_hoisted_function_and_observes_native_dependency() -> TestResult {
    reset();
    let code = concat!(
        "import {css} from '@devup-ui/react';import {token} from './producer';",
        "export const box=css({margin:space()});",
        "function space(){return token+'__leaf__'};throw new Error('host only');"
    );

    let output = run(
        "/view.ts",
        code,
        &[(
            "./producer",
            "/producer.ts",
            "import {createVar} from '@vanilla-extract/css';export const token=createVar();throw new Error('producer host only');",
        )],
    )?;

    assert_eq!(static_value(&output, "margin"), "var(--var-0-0)__leaf__");
    assert!(output.code.contains("function space()"), "{}", output.code);
    assert!(
        output.code.contains("throw new Error(\"host only\")"),
        "{}",
        output.code
    );
    Ok(())
}
