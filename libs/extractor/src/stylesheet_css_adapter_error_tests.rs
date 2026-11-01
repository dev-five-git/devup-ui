use crate::{ExtractOption, ResolvedModule, extract_with_modules};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

#[rstest]
#[case("{color:()=> 'red'}")]
#[case("()=>({color:'red'})")]
#[case("new Map([['color','red']])")]
#[case("new (class {color='red'})()")]
#[case("{color:new Map()}")]
#[case("{color:Symbol('red')}")]
#[case("{color:1n}")]
#[case("(()=>{const rule={};rule.self=rule;return rule})()")]
#[case("{get color(){throw new Error('authored getter')}}")]
#[case("{color:new Date(0)}")]
#[case("{color:{toJSON(){return 'red'}}}")]
#[case("{[Symbol('color')]:'red'}")]
#[case("{color:NaN}")]
#[case("{color:Infinity}")]
#[case("42")]
#[case("css({ p:2 }),{color:()=> 'red'}")]
#[case("(()=>{const rules=[];rules.push(rules);return rules})()")]
#[serial]
fn full_css_adapter_rejects_unsupported_values_at_the_authored_call(
    #[case] value: &str,
    #[values(false, true)] imported: bool,
) -> Result<(), String> {
    // Given: real entry/helper routes, never a JSON.stringify fallback.
    reset_class_map();
    reset_file_map();
    let helper = format!(
        "import {{css}} from '@devup-ui/react';\nexport function make() {{ return css({value}); }}"
    );
    let entry = if imported {
        "import {css} from '@devup-ui/react'; import {make} from './helper';\nconst forceRun = (()=>1)();\nexport const card = make();".to_string()
    } else {
        format!(
            "import {{css}} from '@devup-ui/react';\nconst forceRun = (()=>1)();\nexport const card = css({value});"
        )
    };
    let resolver = move |_: &str, _: &str| {
        Some(ResolvedModule {
            path: "/helper.ts".into(),
            code: helper.clone(),
        })
    };
    // When
    let error = extract_with_modules(
        "/entry.css.ts",
        &entry,
        ExtractOption::default(),
        false,
        &resolver,
    )
    .err()
    .ok_or("unsupported css value must fail")?
    .to_string();
    // Then
    let prefix = if imported {
        "/helper.ts:2:"
    } else {
        "/entry.css.ts:3:"
    };
    assert!(error.starts_with(prefix), "{value}: {error}");
    assert!(error.contains("Fix:"), "{error}");
    assert_eq!(
        error.matches("not a callable function").count(),
        0,
        "{error}"
    );
    assert_eq!(error.matches("__devup_").count(), 0, "{error}");
    Ok(())
}

#[test]
#[serial]
fn full_css_adapter_retains_literal_dead_declaration_origin() -> Result<(), String> {
    // Given
    let source = "import {css} from '@devup-ui/react';\nconst forceRun=(()=>1)();\nexport const card=css({\n  boxAlign:'center'\n});";
    // When
    let error = crate::extract_without_source_map("dead.css.ts", source, ExtractOption::default())
        .err()
        .ok_or("dead declaration must fail")?
        .to_string();
    // Then
    assert!(error.starts_with("dead.css.ts:4:3:"), "{error}");
    assert!(error.contains("boxAlign"), "{error}");
    Ok(())
}

#[test]
#[serial]
fn full_css_adapter_rejects_invalid_rules_when_authored_code_catches_the_error()
-> Result<(), String> {
    // Given
    let source = "import {css} from '@devup-ui/react';\ntry { css({p:()=>2}); } catch(error) { error.message='changed'; error.stack='changed'; }\nexport const card=css({p:2});";
    // When
    let error =
        crate::extract_without_source_map("caught.css.ts", source, ExtractOption::default())
            .err()
            .ok_or("caught css error must remain terminal")?
            .to_string();
    // Then
    assert!(error.starts_with("caught.css.ts:2:"), "{error}");
    assert!(error.contains("css()"), "{error}");
    assert_eq!(error.matches("changed").count(), 0);
    Ok(())
}
