use super::support::*;
use super::*;

#[rstest]
#[case(("export {style as make} from '@vanilla-extract/css';", "3:9:"))]
#[case(("export * from '@vanilla-extract/css';", "3:15:"))]
#[case(("export * as api from '@vanilla-extract/css';", "3:22:"))]
#[case(("export {default} from './terminal';", "3:9:"))]
#[case(("import * as ve from '@vanilla-extract/css';\r\nconst make=ve.style;export {make};", "4:12:"))]
#[serial]
fn raw_terminal_when_binding_extracts_stylesheet_is_located_and_publishes_nothing(
    #[values("css.ts", "css.js")] suffix: &str,
    #[values(true, false)] single: bool,
    #[case] fixture: (&str, &str),
) -> TestResult {
    // Given
    setup();
    native(suffix, single).extract()?;
    let before = export_sheet_internal()?;
    let (body, site) = fixture;
    let path = format!("/b-binding-raw.{suffix}");
    let source = format!("const 한글='😀';\r\n\r\n{body}");
    let terminal = format!("/b-binding-terminal.{suffix}");
    let resolver = move |specifier: &str, _: &str| {
        (specifier == "./terminal").then(|| ResolvedModule {
            path: terminal.clone(),
            code: "import * as ve from '@vanilla-extract/css';export default ve.style;".into(),
        })
    };
    // When
    let error = code_extract_with_modules_internal(
        &path,
        &source,
        "@devup-ui/react",
        "df".into(),
        single,
        false,
        false,
        aliases(),
        &resolver,
    )
    .err()
    .ok_or("binding raw terminal succeeded")?;
    // Then
    assert!(error.contains(&format!("{path}:{site}")), "{error}");
    assert!(
        error.contains("native styling API") && error.contains("escapes"),
        "{error}"
    );
    let fix = error
        .split("Fix:")
        .nth(1)
        .ok_or("direct-import repair missing")?;
    assert!(
        fix.contains("direct")
            && fix.contains("@vanilla-extract/css")
            && fix.contains("consuming stylesheet"),
        "{error}"
    );
    assert_eq!(export_sheet_internal()?, before);
    Ok(())
}
