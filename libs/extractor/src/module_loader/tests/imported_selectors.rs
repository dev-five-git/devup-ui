use super::artifacts::{active_values, exported};
use crate::{ExtractOption, ExtractStyleValue, ResolvedModule, extract_with_modules};
use serial_test::serial;

#[test]
#[serial]
fn imported_registered_styles_remain_selectable_when_d1_removes_their_atoms()
-> Result<(), Box<dyn std::error::Error>> {
    css::file_map::reset_file_map();
    let resolver = |_: &str, _: &str| {
        Some(ResolvedModule {
        path: "/identity.css.ts".into(),
        code: "import {style} from '@devup-ui/react';export const base=style({color:'red',padding:8});export const other=style({color:'red',padding:8});".into(),
    })
    };
    let code = "import {style,globalStyle} from '@devup-ui/react';import {base,other} from './identity.css';export const button=style([base,{color:'blue'}]);export const child=style({selectors:{[`${base}:hover &`]:{color:'green'}}});globalStyle(`${other}:focus`,{opacity:0.5});";
    let output = extract_with_modules(
        "/caller.css.ts",
        code,
        ExtractOption {
            single_css: true,
            ..ExtractOption::default()
        },
        false,
        &resolver,
    )?;
    let classes = exported(&output, "button")?;
    assert_eq!(
        active_values(&output, &classes),
        vec![
            ("color".into(), "blue".into()),
            ("padding".into(), "8px".into())
        ]
    );
    assert!(classes.split_whitespace().any(|class| class == "f0_0"));
    assert!(output.styles.iter().any(|value| matches!(value, ExtractStyleValue::Static(style) if style.selector().is_some_and(|selector| selector.to_string().contains("f0_0:hover")))));
    assert!(output.styles.iter().any(|value| matches!(value, ExtractStyleValue::Static(style) if style.selector().is_some_and(|selector| selector.to_string().contains("f0_1:focus")))));
    Ok(())
}
