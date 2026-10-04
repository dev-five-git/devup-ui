use super::*;

#[test]
fn a_generated_module_is_told_by_its_file() {
    let unit = Unit::generated("/m.css.ts", "export const a: number = 1;");
    assert_eq!(unit.filename(), "/m.css.ts");
    assert_eq!(unit.script(), "export const a = 1;\n");
    assert_eq!(unit.locate(7), None);
    assert_eq!(unit.place(7), "/m.css.ts:1:1");
}

#[test]
fn a_written_module_is_told_down_to_the_token() -> Result<(), String> {
    let code = "const label: string = 'x';\nexport const a: number = fail(label);\n";
    let unit = Unit::written("/m.ts", code, code, &[])?;
    let script = unit.script();
    assert_eq!(
        unit.place(script.find("fail").unwrap_or_default()),
        "/m.ts:2:26"
    );
    assert_eq!(
        unit.place(script.find("(label").unwrap_or_default()),
        "/m.ts:2:30"
    );
    assert_eq!(unit.place(0), "/m.ts:1:1");
    Ok(())
}

#[test]
fn edits_of_the_source_are_undone() -> Result<(), String> {
    let source = "import { style } from '@vanilla-extract/css'; export const a: number = fail();\n";
    let code = "import { style } from '@devup-ui/react'; export const a: number = fail();\n";
    let edits = [(22, 44, 17)];
    let unit = Unit::written("/m.ts", code, source, &[&edits])?;
    let script = unit.script();
    assert_eq!(
        unit.place(script.find("fail").unwrap_or_default()),
        "/m.ts:1:72"
    );
    Ok(())
}

#[test]
fn a_body_is_told_by_the_script_it_was_made_of() -> Result<(), String> {
    let code = "aaaa();\nbbbb();\n";
    let unit = Unit::written("/m.ts", code, code, &[])?;
    let script = unit.script().to_string();
    assert_eq!(script, code);
    let mut body = Body::new(script.len());
    body.copy(&script, 0, 2);
    body.synthesize(3, "XYZ");
    body.copy(&script, 4, 6);
    body.synthesize(8, "\n");
    let (text, origin) = body.finish(&unit);
    assert_eq!(text, "aaXYZ()\n");
    assert_eq!(origin.locate(0).as_deref(), Some("/m.ts:1:1"));
    assert_eq!(origin.locate(1).as_deref(), Some("/m.ts:1:2"));
    assert_eq!(origin.locate(3).as_deref(), Some("/m.ts:1:4"));
    assert_eq!(origin.locate(5).as_deref(), Some("/m.ts:1:5"));
    assert_eq!(origin.locate(6).as_deref(), Some("/m.ts:1:6"));
    assert_eq!(origin.locate(7).as_deref(), Some("/m.ts:2:1"));
    Ok(())
}

#[test]
fn a_script_is_told_in_its_modules_and_not_in_its_glue() -> Result<(), String> {
    let code = "throw 1;\n";
    let unit = Unit::written("/m.ts", code, code, &[])?;
    let mut body = Body::new(code.len());
    body.copy(unit.script(), 0, unit.script().len());
    let (text, origin) = body.finish(&unit);
    let mut script = Script::default();
    script.generated("(function () {\n");
    script.body(&text, &origin);
    script.generated("})();\n");
    assert_eq!(script.text, "(function () {\nthrow 1;\n})();\n");

    let told = script.explain(
        "1 (devup-ui-stylesheet:2:1)\n    at <main> (devup-ui-stylesheet:2:1)\n    at <main> (devup-ui-stylesheet:3:1)",
        "/m.ts",
    );
    assert!(
        told.starts_with("/m.ts:1:1: JS execution error: 1. Fix: "),
        "{told}"
    );
    assert!(told.ends_with("\n    at <main> (/m.ts:1:1)"), "{told}");

    let told = script.explain("Error: x (devup-ui-stylesheet:0:0)", "/m.ts");
    assert!(
        told.starts_with("/m.ts:1:1: JS execution error: Error: x. Fix: "),
        "{told}"
    );
    Ok(())
}
