use oxc_allocator::Allocator;
use oxc_ast::builder::AstBuilder;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;

use super::demand_support::{TestResult, reset, run};
use crate::css_prop::CssProp;
use crate::{ExtractOption, ExtractStyleValue};

#[derive(Clone, Copy, Debug)]
enum Form {
    As,
    Angle,
}

impl Form {
    /// `operand` asserted to `ty`, as an `as` cast or an angle-bracket assertion
    fn wrap(self, operand: &str, ty: &str) -> String {
        match self {
            Self::As => format!("(({operand}) as {ty})"),
            Self::Angle => format!("(<{ty}>({operand}))"),
        }
    }
}

fn object(entries: &str) -> String {
    format!("{{{entries}}}")
}

type Modules = Vec<(&'static str, &'static str, String)>;

const VALUES: &str = "export const space='8px';\nexport const browser=window.document;\n";
const TOKENS: &str = "export const palette={tone:'9px'};\nexport const browser=window.document;\n";

/// The source an asserted pair gives, the modules it imports, and the value
/// its `property` must hold: the same for both forms
fn pair(id: &str, form: Form) -> (String, Modules, &'static str, Vec<&'static str>) {
    let data = |code: &str| vec![("./values", "/values.ts", code.to_string())];
    match id {
        "named_api" => (
            format!(
                "import {{style}} from '@vanilla-extract/css';\nconst make={};\nexport const box=make({{padding:8,color:'teal'}});\n",
                form.wrap("style", "typeof style")
            ),
            vec![],
            "color",
            vec!["teal"],
        ),
        "native_namespace" => (
            format!(
                "import * as ve from '@vanilla-extract/css';\nconst ns={};\nexport const box=ns.style({{margin:3}});\n",
                form.wrap("ve", "typeof ve")
            ),
            vec![],
            "margin",
            vec!["3px"],
        ),
        "namespace_demand" => (
            format!(
                "import {{style}} from '@vanilla-extract/css';\nimport * as data from './values';\nexport const box=style({{margin:{}.space}});\n",
                form.wrap("data", "typeof data")
            ),
            data(VALUES),
            "margin",
            vec!["8px"],
        ),
        "style_table" => (
            format!(
                "import {{css}} from '@devup-ui/react';\nexport function pick(size:string){{return css({{color:{}[size]}});}}\n",
                form.wrap(&object("lg:'large',sm:'small'"), &object("lg:string,sm:string"))
            ),
            vec![],
            "color",
            vec!["large", "small"],
        ),
        "imported_initializer" => (
            "import { css } from '@devup-ui/react';\nimport { gap } from './values';\nexport const box=css({ margin: gap });\n"
                .into(),
            data(&format!("export const gap={};\n", form.wrap("'6px'", "string"))),
            "margin",
            vec!["6px"],
        ),
        "loader_demand" => (
            format!(
                "import {{style}} from '@vanilla-extract/css';\nimport * as ns from './values';\nconst untouched=window.document;\nexport const box=style({{margin:{}.palette.tone}});\n",
                form.wrap("ns", "typeof ns")
            ),
            data(TOKENS),
            "margin",
            vec!["9px"],
        ),
        "escape_holder" => (
            "import { css } from '@devup-ui/react';\nimport { base } from './values';\nexport const box=css({ margin: base.inner.space });\n"
                .into(),
            data(&format!(
                "export const base={{inner:{{space:'8px'}}}};\nexport const copy={};\n",
                form.wrap(&object("held:base"), &object("held:typeof base"))
            )),
            "margin",
            vec!["8px"],
        ),
        _ => unreachable!("unknown assertion pair {id}"),
    }
}

#[rstest]
#[case::named_api("named_api")]
#[case::native_namespace("native_namespace")]
#[case::namespace_demand("namespace_demand")]
#[case::style_table("style_table")]
#[case::imported_initializer("imported_initializer")]
#[case::loader_demand("loader_demand")]
#[case::escape_holder("escape_holder")]
#[serial]
fn angle_assertion_gives_what_the_as_cast_gives(
    #[case] id: &str,
    #[values(Form::As, Form::Angle)] form: Form,
) -> TestResult {
    // Given
    reset();
    let (source, modules, property, expected) = pair(id, form);
    let modules: Vec<_> = modules
        .iter()
        .map(|(name, path, code)| (*name, *path, code.as_str()))
        .collect();
    // When
    let output = run("/assertion.ts", &source, &modules)?;
    // Then
    let mut actual: Vec<&str> = output
        .styles
        .iter()
        .filter_map(|style| match style {
            ExtractStyleValue::Static(style) if style.property == property => {
                Some(style.value.as_str())
            }
            _ => None,
        })
        .collect();
    actual.sort_unstable();
    assert_eq!(actual, expected, "{}", output.code);
    Ok(())
}

#[rstest]
fn constant_fold_reads_an_angle_assertion_like_the_as_cast(
    #[values(Form::As, Form::Angle)] form: Form,
) {
    // Given
    let source = format!(
        "import {{css}} from '@devup-ui/react';\nconst a=2,b=3;\nexport const box=css({{margin:{}+{}}});\n",
        form.wrap("a", "number"),
        form.wrap("b", "number")
    );
    let allocator = Allocator::default();
    let mut program = Parser::new(&allocator, &source, SourceType::ts())
        .parse()
        .program;
    // When
    crate::imported_constants::inline_constants(
        &AstBuilder::new(&allocator),
        &mut program,
        "/assertion.ts",
        &ExtractOption::default(),
        None,
        CssProp::Off,
    );
    // Then
    let code = Codegen::new().build(&program).code;
    assert_eq!(code.matches("margin: 5").count(), 1, "{code}");
}
