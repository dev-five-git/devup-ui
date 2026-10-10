#[cfg(test)]
use super::DevupVisitor;
#[cfg(test)]
use oxc_allocator::GetAllocator;
#[cfg(test)]
use oxc_ast::ast::{Expression, ObjectPropertyKind};

mod source_capture;

#[cfg(test)]
mod literal_w38f_wrapper;

#[cfg(test)]
mod literal_w38b_sources {
    use super::*;
    use oxc_allocator::CloneIn;
    use oxc_ast::ast::Statement;
    use oxc_ast_visit::VisitMut;
    use serial_test::serial;

    #[test]
    #[serial]
    fn callback_when_nested_text_is_lowered_preserves_real_root_spread() {
        // Given: an actual mixed object, with a nested template lowered by production.
        let source = "({...rest,_hover:`style-order:${p=>p.active?2:3};color:red`});";
        let allocator = oxc_allocator::Allocator::default();
        let mut parsed =
            oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::tsx()).parse();
        assert_eq!(parsed.diagnostics.len(), 0);
        let Statement::ExpressionStatement(statement) = &mut parsed.program.body[0] else {
            panic!("expression fixture")
        };
        let mut visitor = DevupVisitor::new(&allocator, "a.tsx", "@devup-ui/react", vec![], None);
        let value = crate::utils::unwrap_syntax_only_mut(&mut statement.expression);
        assert!(crate::css_utils::literal_tree::lower(
            &visitor.ast,
            value,
            crate::css_utils::literal_tree::Scope {
                source: Some(source),
                global: false
            }
        ));
        let mut captures = Vec::new();
        let mut renders = Vec::new();
        // When: the real literal callback preparer receives the produced mixed container.
        let actual = visitor.capture_literal_styled(value, &mut captures, &mut renders);
        // Then: the source spread survives; a nested callback is prepared once.
        assert!(actual);
        let Expression::ObjectExpression(object) = value else {
            panic!("mixed object")
        };
        assert!(matches!(
            object.properties[0],
            ObjectPropertyKind::SpreadProperty(_)
        ));
        assert_eq!(renders.len(), 1);
        assert_eq!(visitor.errors.len(), 0);
    }

    #[test]
    #[serial]
    fn order_wrapper_when_callback_produces_classes_observes_finite_provenance() {
        // Given: public visiting first processes a real class-producing callback.
        let source = "import {css} from '@devup-ui/react';p=>css({color:p.active?'red':'blue'});";
        let allocator = oxc_allocator::Allocator::default();
        let mut parsed =
            oxc_parser::Parser::new(&allocator, source, oxc_span::SourceType::tsx()).parse();
        assert_eq!(parsed.diagnostics.len(), 0);
        let mut visitor = DevupVisitor::new(&allocator, "a.tsx", "@devup-ui/react", vec![], None);
        visitor.visit_program(&mut parsed.program);
        let Statement::ExpressionStatement(statement) = parsed
            .program
            .body
            .last_mut()
            .unwrap_or_else(|| panic!("retained expression"))
        else {
            panic!("callback fixture")
        };
        let mut order_callback = statement
            .expression
            .clone_in_with_semantic_ids(visitor.ast.allocator());
        assert!(super::super::literal_callback_order::wrap(
            &visitor.ast,
            &mut statement.expression
        ));
        let mut captures = Vec::new();
        // When: the actual wrapped callback passes through the production preparer.
        let (parts, _) = visitor
            .prepare_callback(&mut statement.expression, &mut captures)
            .unwrap_or_else(|| panic!("prepared wrapper"));
        // Then: finite provenance is observed, not a manufactured Class part.
        assert!(
            matches!(parts.as_slice(), [crate::composition::KnownPart::Styles(styles)] if matches!(styles.as_slice(), [crate::composition::KnownStyles::Finite(_, _)]))
        );
        let wrapped =
            super::super::literal_callback_order::wrapped(&visitor.ast, &mut order_callback)
                .unwrap_or_else(|| panic!("source wrapper"));
        let (parts, _) = visitor
            .prepare_order_callback(&mut order_callback, &mut Vec::new(), wrapped)
            .unwrap_or_else(|| panic!("typed source wrapper"));
        assert!(super::super::literal_callback_order::value(&visitor.ast, &parts).is_none());
    }
}
