use crate::ExtractStyleValue;
use crate::extract_style::extract_static_style::ExtractStaticStyle;
use crate::extractor::extract_style_from_expression::{
    LiteralHandling, extract_style_from_expression,
};
use crate::finite_styles::FiniteStyles;
use crate::gen_class_name::gen_class_names;
use crate::utils::unwrap_syntax_only;
use oxc_allocator::{Allocator, CloneIn};
use oxc_ast::{
    ast::{Expression, Statement},
    builder::AstBuilder,
};
use oxc_parser::Parser;
use oxc_span::SourceType;
use rstest::rstest;
use serial_test::serial;
use std::error::Error;

/// One lookup whose key `b` selects a value that sets no declaration.
const LOOKUP: &str = "({color:({a:'red',b:null})[key]});";

/// Puts the process-wide debug flag back, also when an assertion fails.
struct Readable(bool);

impl Readable {
    fn enabled() -> Self {
        let previous = css::debug::is_debug();
        css::debug::set_debug(true);
        Self(previous)
    }
}

impl Drop for Readable {
    fn drop(&mut self) {
        css::debug::set_debug(self.0);
    }
}

fn parsed<'a>(allocator: &'a Allocator, source: &'a str) -> Result<Expression<'a>, Box<dyn Error>> {
    let mut parsed = Parser::new(allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let Some(Statement::ExpressionStatement(statement)) = parsed.program.body.pop() else {
        return Err("an expression statement".into());
    };
    Ok(unwrap_syntax_only(&statement.expression).clone_in(allocator))
}

fn atom(property: &str, value: &str) -> ExtractStyleValue {
    ExtractStyleValue::Static(ExtractStaticStyle::new(property, value, 0, None))
}

#[rstest]
#[case::selected_key_without_class(&[("", None), ("color-0-red--255", Some(("color", "red")))])]
#[serial]
fn w38r_u7_outcomes_when_a_selected_key_emits_no_class_reads_it_as_empty(
    #[case] expected: &[(&str, Option<(&str, &str)>)],
) -> Result<(), Box<dyn Error>> {
    // Given: the lookup object omits key `b`, because its value emits no class.
    let _readable = Readable::enabled();
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let mut value = parsed(&allocator, LOOKUP)?;
    let mut extracted = extract_style_from_expression(
        &ast,
        None,
        &mut value,
        0,
        &None,
        LiteralHandling::ExpandResponsiveThemeToken,
    );
    let emitted = gen_class_names(&ast, &mut extracted.styles, None, None)
        .ok_or("the lookup emits a class")?;
    // When: the finite outcomes of the emitted lookup are read for every key.
    let finite = FiniteStyles::emitted(&ast, &extracted.styles, &emitted)
        .ok_or("the emitted lookup yields finite outcomes")?;
    // Then: key `b` selects no class and sets nothing, like the unset key.
    let expected: Vec<(String, Vec<ExtractStyleValue>)> = expected
        .iter()
        .map(|(text, declarations)| {
            (
                (*text).to_string(),
                declarations
                    .iter()
                    .map(|(property, value)| atom(property, value))
                    .collect(),
            )
        })
        .collect();
    assert_eq!(finite.results, expected);
    Ok(())
}
