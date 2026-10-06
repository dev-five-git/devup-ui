use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_parser::Parser;
use oxc_semantic::SemanticBuilder;
use oxc_span::{GetSpan, SourceType};

use crate::evaluation_sandbox::Failure;
use crate::module_loader::{Script, operations::Operations};

pub(crate) fn required(
    failure: &Failure,
    script: &Script,
    operations: &Operations,
) -> Option<String> {
    let violations = match failure {
        Failure::Js(_) => return None,
        Failure::Forbidden(violations) => violations,
    };
    let messages: Vec<_> = violations.iter().filter_map(|violation| {
        let (_, offset) = violation.site()?;
        let original = script.original(operations.original_offset(offset))?;
        let allocator = Allocator::default();
        let parsed = Parser::new(&allocator, original.source,
            SourceType::from_path(original.filename).unwrap_or_default()).parse();
        let semantic = SemanticBuilder::new().with_build_nodes(true).build(&parsed.program).semantic;
        let node = semantic.nodes().iter().find(|node| matches!(node.kind(), AstKind::IdentifierReference(_))
            && node.kind().span().start <= u32::try_from(original.offset).unwrap_or(u32::MAX)
            && original.offset < usize::try_from(node.kind().span().end).unwrap_or(original.source.len()))?;
        let mut span = node.kind().span();
        for parent in semantic.nodes().ancestor_kinds(node.id()) {
            match parent {
                AstKind::StaticMemberExpression(member) if member.object.span() == span => span = member.span,
                AstKind::ComputedMemberExpression(member) if member.object.span() == span => span = member.span,
                AstKind::ParenthesizedExpression(_) | AstKind::TSAsExpression(_)
                | AstKind::TSSatisfiesExpression(_) | AstKind::TSNonNullExpression(_)
                | AstKind::TSTypeAssertion(_) | AstKind::ChainExpression(_) => span = parent.span(),
                _ => break,
            }
        }
        Some(format!("{}: native styling cannot use `{}` at build time: this required expression needs an exact, static input. Fix: provide an exact static value or CSS variable for this expression",
            original.place(), span.source_text(original.source)))
    }).collect();
    (!messages.is_empty()).then(|| messages.join("\n"))
}
