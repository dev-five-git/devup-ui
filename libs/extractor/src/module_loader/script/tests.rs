use super::*;
use oxc_ast_visit::{Visit, walk};
use oxc_span::SourceType;

#[derive(Default)]
struct Calls(Vec<usize>);

impl<'a> Visit<'a> for Calls {
    fn visit_call_expression(&mut self, call: &oxc_ast::ast::CallExpression<'a>) {
        self.0.push(call.span.start as usize);
        walk::walk_call_expression(self, call);
    }
}

#[rstest::rstest]
#[case("()=>{}")]
#[case("function(){}")]
#[case("(value)=>value")]
#[case("async()=>{}")]
fn anonymous_calls_keep_original_expression_starts_when_formatted(
    #[case] function: &str,
) -> Result<(), String> {
    // Given
    let source =
        format!("const label: string='a  b';\nexport const value=({function}).read('key');");
    let unit = Unit::written("anonymous.ts", &source, &source, &[])?;
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, unit.script(), SourceType::mjs()).parse();
    let mut calls = Calls::default();
    calls.visit_program(&parsed.program);
    let at = *calls.0.first().ok_or("missing call")?;
    let expected = source.find(function).ok_or("missing original function")?;
    // When
    let location = unit.place(at);
    // Then
    assert_eq!(location, crate::locate("anonymous.ts", &source, expected));
    Ok(())
}

#[rstest::rstest]
fn anonymous_calls_keep_distinct_original_sites_when_prior_layers_change(
    #[values("\n", "\r\n", "\r", "\u{2028}", "\u{2029}")] newline: &str,
) -> Result<(), String> {
    // Given
    let source = format!(
        "import {{style}} from '@vanilla-extract/css';{newline}const label: string='😀  한';{newline}export const value=style({{first:(()=>{{}}).read('key')?'a  b':'c',second:(()=>{{}}).read('key')?'d':'e'}});"
    );
    let aliased = source.replace("@vanilla-extract/css", "@devup-ui/react");
    let alias_start = source
        .find("@vanilla-extract/css")
        .ok_or("missing import")?;
    let aliases = [(
        alias_start,
        alias_start + "@vanilla-extract/css".len(),
        "@devup-ui/react".len(),
    )];
    let prefix = "const earlier: number=0;\n";
    let code = format!("{prefix}{aliased}");
    let earlier = [(0, 0, prefix.len())];
    let unit = Unit::written("duplicates.ts", &code, &source, &[&earlier, &aliases])?;
    let allocator = oxc_allocator::Allocator::default();
    let parsed = oxc_parser::Parser::new(&allocator, unit.script(), SourceType::mjs()).parse();
    let mut calls = Calls::default();
    calls.visit_program(&parsed.program);
    let expected: Vec<_> = source
        .match_indices("()=>{}")
        .map(|(at, _)| crate::locate("duplicates.ts", &source, at))
        .collect();
    // When
    let locations: Vec<_> = calls.0.iter().skip(1).map(|&at| unit.place(at)).collect();
    // Then
    assert_eq!(locations, expected);
    Ok(())
}

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
