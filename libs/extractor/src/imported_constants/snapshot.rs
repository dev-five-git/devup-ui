//! An eagerly copied scalar is independent of subsequent object mutation.

use super::{ChangeSite, ModuleScope, provenance};
use oxc_ast::AstKind;
use oxc_ast::ast::Expression;
use oxc_semantic::SemanticBuilder;
use oxc_span::GetSpan;

impl ModuleScope<'_, '_> {
    pub(super) fn snapshot_initializer(&self, init: &Expression<'_>) -> Option<u32> {
        if !matches!(
            crate::utils::unwrap_syntax_only(init),
            Expression::StaticMemberExpression(_) | Expression::ComputedMemberExpression(_)
        ) {
            return None;
        }
        let semantic = SemanticBuilder::new()
            .with_build_nodes(true)
            .build(self.program)
            .semantic;
        let proof = provenance::Proof {
            nodes: semantic.nodes(),
            scoping: semantic.scoping(),
        };
        matches!(proof.expression(init), provenance::Shape::Primitive).then_some(init.span().start)
    }

    pub(super) fn primitive_snapshot(&self, name: &str) -> bool {
        let (Some(snapshot), Some(hazards)) = (self.snapshot_at, self.hazards.get(name)) else {
            return false;
        };
        let semantic = SemanticBuilder::new()
            .with_build_nodes(true)
            .build(self.program)
            .semantic;
        hazards.iter().all(|change| {
            let ChangeSite::Here(hazard) = change.site else {
                return false;
            };
            snapshot < hazard
                && semantic
                    .nodes()
                    .iter()
                    .filter(|node| node.kind().span().start == hazard)
                    .all(|node| {
                        !semantic.nodes().ancestor_kinds(node.id()).any(|kind| {
                            matches!(
                                kind,
                                AstKind::Function(_)
                                    | AstKind::ArrowFunctionExpression(_)
                                    | AstKind::Class(_)
                            )
                        })
                    })
        })
    }
}
