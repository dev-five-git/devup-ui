use css::{class_map::reset_class_map, file_map::reset_file_map};
use oxc_allocator::Allocator;
use oxc_ast_visit::VisitMut;
use oxc_codegen::Codegen;
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::visit::DevupVisitor;

mod cjs_aliases;
mod class_names;
mod components;
mod css_bindings;
mod diagnostics;
mod imports;
mod lowered_reads;
mod preflight_contexts;
mod scoping;
mod stylex;
mod stylex_boundary_coverage;
mod stylex_entrypoint_controls;
mod stylex_entrypoints;
mod stylex_loader_patterns;
mod stylex_residuals;
mod theme_reads;

struct Visited {
    code: String,
    errors: Vec<String>,
    styles: usize,
}

fn visit_with(code: &str, setup: impl FnOnce(&mut DevupVisitor<'_>)) -> Visited {
    reset_class_map();
    reset_file_map();
    let allocator = Allocator::default();
    let mut program = Parser::new(&allocator, code, SourceType::tsx())
        .parse()
        .program;
    let mut visitor =
        DevupVisitor::new(&allocator, "test.tsx", "@devup-ui/react", Vec::new(), None);
    setup(&mut visitor);
    visitor.visit_program(&mut program);
    let printed = Codegen::new().build(&program).code;
    Visited {
        code: printed.split_whitespace().collect::<Vec<_>>().join(" "),
        errors: visitor
            .errors
            .iter()
            .map(|(_, message)| message.clone())
            .collect(),
        styles: visitor.styles.len(),
    }
}

fn visit(code: &str) -> Visited {
    visit_with(code, |_| {})
}

const IMPORT_CSS: &str = "import { css } from '@devup-ui/react';\n";

fn extracted(code: &str) -> Result<String, String> {
    reset_class_map();
    reset_file_map();
    crate::extract(
        "test.tsx",
        code,
        crate::ExtractOption {
            import_main_css: false,
            ..crate::ExtractOption::default()
        },
    )
    .map(|output| output.code.split_whitespace().collect::<Vec<_>>().join(" "))
    .map_err(|error| error.to_string())
}
