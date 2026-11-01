use crate::{ExtractOption, ExtractOutput, extract_without_source_map};
use css::{class_map::reset_class_map, file_map::reset_file_map};
use rstest::rstest;
use serial_test::serial;

fn compile(source: &str) -> Result<ExtractOutput, String> {
    reset_class_map();
    reset_file_map();
    extract_without_source_map("/gate.css.ts", source, ExtractOption::default())
        .map_err(|error| error.to_string())
}

#[rstest]
#[case("{skip:()=>1,nested:new Date(0)}")]
#[case("[()=>1,{nested:new Date(0)}]")]
#[case("{first:{skip:()=>1},nested:[new Date(0)]}")]
#[case("{get skip(){return ()=>1},nested:new Date(0)}")]
#[serial]
fn gate_boundaries_reject_date_after_unsupported_export_at_creation(
    #[case] value: &str,
) -> Result<(), String> {
    // Given
    let source = format!("import{{style}}from'@devup-ui/react';export const result={value};");
    let offset = source.find("Date(0)").ok_or("missing creation")?;
    // When
    let error = compile(&source)
        .err()
        .ok_or("Date escaped through initializer fallback")?;
    // Then
    assert!(
        error.starts_with(&format!(
            "{}:",
            crate::locate("/gate.css.ts", &source, offset)
        )),
        "{error}"
    );
    assert!(error.contains("ReferenceError: `Date`"), "{error}");
    Ok(())
}

#[rstest]
#[case("const base=css({color:'red'});export const card=css({selectors:{[`.${base} &`]:{p:2}}});")]
#[case(
    "const base=css({p:1,color:'red'});const extra=css({opacity:0.5});const joined=css(base,extra);export const card=css({selectors:{[`.raw .${joined}:hover > .${base} &`]:{p:2},[`.${extra} & + .tail`]:{m:1}},[`.${base} &`]:{w:2}});"
)]
#[serial]
fn gate_boundaries_computed_selector_keys_match_plain_classes(
    #[case] body: &str,
) -> Result<(), String> {
    // Given
    let plain = format!("import{{css}}from'@devup-ui/react';{body}");
    reset_class_map();
    reset_file_map();
    let expected = extract_without_source_map("/gate.tsx", &plain, ExtractOption::default())
        .map_err(|error| error.to_string())?;
    let full = format!("import{{css}}from'@devup-ui/react';const forceRun=(()=>1)();{body}");
    assert_eq!(
        crate::stylesheet_policy::plan(
            &full,
            "/gate.css.ts",
            &ExtractOption::default(),
            None,
            &|_| false
        ),
        crate::stylesheet_policy::Plan::Run
    );
    // When
    let actual = compile(&full)?;
    // Then
    let constants = |code: &str| {
        code.lines()
            .filter(|line| line.contains("const ") && !line.contains("forceRun"))
            .map(str::to_string)
            .collect::<Vec<_>>()
    };
    assert_eq!(constants(&actual.code), constants(&expected.code));
    assert_eq!(actual.styles, expected.styles);
    Ok(())
}

#[rstest]
#[case("import*as ui from'@devup-ui/react';", "ui['css']")]
#[case("import*as ui from'@devup-ui/react';", "ui[`css`]")]
#[case(
    "import*as ui from'@devup-ui/react';const ns=ui;const paint=ns['css'];",
    "paint"
)]
#[serial]
fn gate_boundaries_computed_namespace_tag_rejection_survives_catch(
    #[case] import: &str,
    #[case] tag: &str,
    #[values(false, true)] mutate: bool,
) -> Result<(), String> {
    // Given
    let caught = if mutate {
        "catch(error){error.message='changed';error.stack='changed';}"
    } else {
        "catch{}"
    };
    let source = format!(
        "{import}const forceRun=(()=>1)();try{{{tag}`color:${{()=> 'red'}};`;}}{caught}export const card=ui.css({{p:2}});"
    );
    let offset = source.find(&format!("{tag}`color")).ok_or("missing tag")?;
    // When
    let error = compile(&source)
        .err()
        .ok_or("caught computed tag escaped")?;
    // Then
    assert!(
        error.starts_with(&format!(
            "{}:",
            crate::locate("/gate.css.ts", &source, offset)
        )),
        "{error}"
    );
    assert!(error.contains("css()") && error.contains("Fix:"), "{error}");
    assert_eq!(error.matches("changed").count(), 0);
    Ok(())
}

#[test]
#[serial]
fn gate_boundaries_composition_preserves_array_getters_and_shared_object_reads()
-> Result<(), String> {
    // Given
    let source = "import{css}from'@devup-ui/react';let events='';let reads=0;const shared={get p(){events+='p';return ++reads}};const inner=[];Object.defineProperty(inner,0,{get(){events+='i';return shared}});const outer=[];Object.defineProperty(outer,0,{get(){events+='a';return shared}});Object.defineProperty(outer,1,{get(){events+='b';return inner}});export const card=css(outer);export const trace=events;export const count=reads;";
    // When
    let output = compile(source)?;
    // Then
    assert!(
        output.code.contains("export const trace = \"abpip\""),
        "{}",
        output.code
    );
    assert!(
        output.code.contains("export const count = 2"),
        "{}",
        output.code
    );
    assert!(output.styles.iter().any(|value| matches!(value, crate::ExtractStyleValue::Static(style) if style.property() == "padding" && style.value() == "8px")), "{:?}", output.styles);
    Ok(())
}
