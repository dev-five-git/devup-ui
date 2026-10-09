use super::*;

mod classes;
mod generic;
mod merging;
mod typography;
mod w38i_a;
mod w38i_b;

fn parsed<'a>(allocator: &'a Allocator, source: &'a str) -> Expression<'a> {
    let mut parsed = oxc_parser::Parser::new(allocator, source, SourceType::tsx()).parse();
    assert_eq!(parsed.diagnostics.len(), 0);
    let semantic = oxc_semantic::SemanticBuilder::new().build(&parsed.program);
    assert_eq!(semantic.diagnostics.len(), 0);
    let Statement::ExpressionStatement(statement) = parsed
        .program
        .body
        .pop()
        .unwrap_or_else(|| panic!("expression"))
    else {
        panic!("parsed expression statement required")
    };
    unwrap_syntax_only(&statement.expression).clone_in(allocator)
}

fn json_evaluated(value: &Expression<'_>, setup: &str) -> serde_json::Value {
    let source = format!("JSON.stringify(({}));", readable_code(value));
    let allocator = Allocator::default();
    let serialized = parsed(&allocator, &source);
    let actual = evaluated(&serialized, setup);
    let value: serde_json::Value = serde_json::from_str(
        actual[0]
            .as_str()
            .unwrap_or_else(|| panic!("serialized expression value")),
    )
    .unwrap_or_else(|error| panic!("expression JSON: {error}"));
    serde_json::json!([value, actual[1]])
}

struct DebugMode(bool);

impl DebugMode {
    fn enabled() -> Self {
        let previous = css::debug::is_debug();
        css::debug::set_debug(true);
        Self(previous)
    }
}

impl Drop for DebugMode {
    fn drop(&mut self) {
        css::debug::set_debug(self.0);
    }
}

fn class_of(style: &ExtractStyleValue) -> String {
    match style
        .extract(None)
        .unwrap_or_else(|| panic!("emitting declaration"))
    {
        StyleProperty::ClassName(value) => value,
        StyleProperty::Variable { class_name, .. } => class_name,
    }
}
