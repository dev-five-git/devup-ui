use super::demand_support::{TestResult, reset};
use super::mixed_support::{assert_consumed, has_static};
use super::*;

#[rstest]
#[case(("export {style} from '@vanilla-extract/css';", "style}", "import {style as make} from './api';", "make"))]
#[case(("export {style as make} from '@vanilla-extract/css';", "style as", "import {make} from './api';", "make"))]
#[case(("export * from '@vanilla-extract/css';", "'@vanilla-extract/css'", "import {style as make} from './api';", "make"))]
#[case(("export * as api from '@vanilla-extract/css';", "'@vanilla-extract/css'", "import {api} from './api';", "api.style"))]
#[case(("export {default} from './terminal';", "default}", "import make from './api';", "make"))]
#[case(("export {make} from './middle';", "make}", "import {make} from './api';", "make"))]
#[case(("import * as ve from '@vanilla-extract/css';const make=ve.style;export {make};", "ve.style", "import {make} from './api';", "make"))]
#[case(("import * as ve from '@vanilla-extract/css';export default ve.style;", "ve.style", "import make from './api';", "make"))]
#[case(("import * as ve from '@vanilla-extract/css';const key='style';const make=ve[key];export {make};", "ve[key]", "import {make} from './api';", "make"))]
#[case((concat!("import * as ve from '@vanilla-extract/css';", "const {style:make}=ve;export {make};"), "ve;", "import {make} from './api';", "make"))]
#[serial]
fn raw_native_terminal_when_no_internal_demand_exists_has_original_site_and_direct_import_fix(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
    #[case] fixture: (&str, &str, &str, &str),
) -> TestResult {
    // Given
    reset();
    let (body, site, _, _) = fixture;
    let source = format!("const 한글='😀';\r\n\r\n{body}");
    let path = format!("/b-raw.{suffix}");
    let place = crate::locate(
        &path,
        &source,
        source.find(site).ok_or("native export site missing")?,
    );
    let terminal_path = format!("/b-terminal.{suffix}");
    let middle_path = format!("/b-middle.{suffix}");
    let resolver = move |specifier: &str, _: &str| match specifier {
        "./terminal" => Some(ResolvedModule {
            path: terminal_path.clone(),
            code: "import * as ve from '@vanilla-extract/css';export default ve.style;".into(),
        }),
        "./middle" => Some(ResolvedModule {
            path: middle_path.clone(),
            code: "export {style as make} from '@vanilla-extract/css';".into(),
        }),
        _ => None,
    };
    let mut option = option();
    option.single_css = single;
    // When
    let error = extract_with_modules(&path, &source, option, true, &resolver)
        .err()
        .ok_or("raw native reexport succeeded")?
        .to_string();
    // Then
    assert!(error.contains(&place), "{error}");
    assert!(
        error.contains("native styling API") && error.contains("escapes"),
        "{error}"
    );
    let fix = error
        .split("Fix:")
        .nth(1)
        .ok_or("native export repair missing")?;
    assert!(
        fix.contains("direct")
            && fix.contains("@vanilla-extract/css")
            && fix.contains("consuming stylesheet"),
        "{error}"
    );
    assert!(!error.contains("Invalid exports"), "{error}");
    Ok(())
}

#[rstest]
#[case(("export {style} from '@vanilla-extract/css';", "import {style as make} from './api';", "make"))]
#[case(("export {style as make} from '@vanilla-extract/css';", "import {make} from './api';", "make"))]
#[case(("export * from '@vanilla-extract/css';", "import {style as make} from './api';", "make"))]
#[case(("export * as api from '@vanilla-extract/css';", "import {api} from './api';", "api.style"))]
#[case(("export {default} from './terminal';", "import make from './api';", "make"))]
#[case(("export {make} from './middle';", "import {make} from './api';", "make"))]
#[case(("import * as ve from '@vanilla-extract/css';const make=ve.style;export {make};", "import {make} from './api';", "make"))]
#[case(("import * as ve from '@vanilla-extract/css';export default ve.style;", "import make from './api';", "make"))]
#[case(("import * as ve from '@vanilla-extract/css';const key='style';const make=ve[key];export {make};", "import {make} from './api';", "make"))]
#[case((concat!("import * as ve from '@vanilla-extract/css';", "const {style:make}=ve;export {make};"), "import {make} from './api';", "make"))]
#[serial]
fn same_native_terminal_when_internal_exact_demand_exists_is_consumed(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
    #[case] fixture: (&str, &str, &str),
) -> TestResult {
    // Given
    reset();
    let (carrier, import, callee) = fixture;
    let carrier_path = format!("/b-internal.{suffix}");
    let carrier = carrier.to_string();
    let resolver = move |specifier: &str, _: &str| match specifier {
        "./api" => Some(ResolvedModule {
            path: carrier_path.clone(),
            code: carrier.clone(),
        }),
        "./terminal" => Some(ResolvedModule {
            path: "/b-terminal.ts".into(),
            code: "import * as ve from '@vanilla-extract/css';export default ve.style;".into(),
        }),
        "./middle" => Some(ResolvedModule {
            path: "/b-middle.ts".into(),
            code: "export {style as make} from '@vanilla-extract/css';".into(),
        }),
        _ => None,
    };
    let source = format!(
        "{import}export const box={callee}({{color:'blue',padding:8}});const browser=window.document;"
    );
    let mut option = option();
    option.single_css = single;
    // When
    let output = extract_with_modules(
        &format!("/b-demand.{suffix}"),
        &source,
        option,
        false,
        &resolver,
    )?;
    // Then
    assert_consumed(&output);
    assert!(has_static(&output, "color", "blue") && has_static(&output, "padding", "8px"));
    assert!(!has_static(&output, "padding", "32px"));
    assert!(!output.code.contains(&format!("{callee}(")));
    assert!(output.code.contains("./api") && output.code.contains("window.document"));
    assert!(
        output
            .dependencies
            .contains(&format!("/b-internal.{suffix}"))
    );
    Ok(())
}

#[rstest]
#[case("export const style=()=>7;")]
#[case("import {style as unused} from '@vanilla-extract/css';export const helper=()=>7;")]
#[case("import * as unused from '@vanilla-extract/css';export const helper=()=>7;")]
#[serial]
fn ordinary_export_when_native_terminal_is_absent_keeps_devup_fallback(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
    #[case] control: &str,
) -> TestResult {
    // Given
    reset();
    let source = format!(
        "{control}import {{css}} from '@devup-ui/react';export const box=css({{color:'orange',p:2}});"
    );
    let mut option = option();
    option.single_css = single;
    // When
    let output = extract_with_modules(
        &format!("/b-ordinary.{suffix}"),
        &source,
        option,
        false,
        &|_, _| None,
    )?;
    // Then
    assert!(has_static(&output, "color", "orange") && has_static(&output, "padding", "8px"));
    assert!(!has_static(&output, "padding", "32px"));
    assert!(output.code.contains('7'));
    Ok(())
}

#[rstest]
#[serial]
fn type_only_export_when_typescript_stylesheet_is_extracted_is_not_native_escape(
    #[values(true, false)] single: bool,
) -> TestResult {
    // Given
    reset();
    let source = "export type {StyleRule} from '@vanilla-extract/css';import {css} from '@devup-ui/react';export const helper=()=>7;export const box=css({color:'orange',p:2});";
    let mut option = option();
    option.single_css = single;
    // When
    let output = extract_with_modules("/b-type-only.css.ts", source, option, false, &|_, _| None)?;
    // Then
    assert!(has_static(&output, "color", "orange") && has_static(&output, "padding", "8px"));
    assert!(output.code.contains('7'));
    Ok(())
}
