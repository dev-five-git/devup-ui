//! Final-JavaScript guard facts collected directly from the Oxc AST.

mod callable;
#[cfg(test)]
mod callable_tests;
#[cfg(test)]
mod coverage_helper_tests;
#[cfg(test)]
mod coverage_reference_tests;
#[cfg(test)]
mod coverage_scope_tests;
mod diagnostics;
mod exports;
#[cfg(test)]
mod observation_tests;
mod origins;
mod patterns;
#[cfg(test)]
mod reference_tests;
mod scopes;
#[cfg(test)]
mod tests;
mod text;
mod traversal;
mod wire;
mod writes;

use std::{error::Error, fmt};

use oxc_allocator::Allocator;
use oxc_ast_visit::Visit;
use oxc_parser::{ParseOptions, Parser};
use oxc_span::SourceType;

use scopes::Scopes;
use traversal::Collector;
use wire::Positions;

/// Failure to parse final JavaScript.
#[derive(Debug)]
pub enum CompiledGuardError {
    /// Located parser diagnostics in final compiled output.
    Parse(Vec<String>),
}

impl fmt::Display for CompiledGuardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Parse(errors) => formatter.write_str(&errors.join("\n")),
        }
    }
}

impl Error for CompiledGuardError {}

/// Produce ordered guard facts for final JavaScript, allowing JSX but not TypeScript.
///
/// Parentheses follow the current Vite public parser: no wrapper nodes. Positions
/// on the wire are JavaScript UTF-16 offsets. This never executes source or
/// accesses extraction/CSS state.
///
/// # Errors
/// Returns located parse diagnostics rather than facts for invalid JavaScript.
pub fn compiled_guard_facts_internal(
    code: &str,
    observe: bool,
) -> Result<String, CompiledGuardError> {
    let allocator = Allocator::default();
    let parsed = Parser::new(&allocator, code, SourceType::jsx())
        .with_options(ParseOptions {
            preserve_parens: false,
            ..ParseOptions::default()
        })
        .parse();
    let positions = Positions::new(code);
    let errors: Vec<_> = parsed
        .diagnostics
        .errors()
        .map(|error| {
            let offset = error
                .labels
                .iter()
                .find(|label| label.primary())
                .or_else(|| error.labels.first())
                .map_or(0, oxc_span::LabeledSpan::offset);
            let (line, column) = diagnostics::location(code, offset);
            format!("compiled output:{line}:{column}: {error}")
        })
        .collect();
    if !errors.is_empty() {
        return Err(CompiledGuardError::Parse(errors));
    }
    let scopes = Scopes::build(&parsed.program);
    let mut collector = Collector::new(scopes, positions, observe);
    collector.prepare(&parsed.program);
    collector.visit_program(&parsed.program);
    collector.finish(&parsed.program);
    Ok(collector.facts.json())
}
