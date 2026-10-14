use oxc_allocator::Allocator;
use oxc_ast::{ast::Expression, builder::AstBuilder};
use oxc_span::Span;

use super::{SiteScope, numeric_role, plan_numeric_roles, retain_folded_owner};

#[test]
fn numeric_unit_groups_follow_target_order_when_one_site_has_mixed_units() {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = Expression::new_identifier(Span::new(4, 5), "n", &ast);
    let _scope = SiteScope::enter("source.tsx", "abcdefghijkl", &[]);
    plan_numeric_roles(
        &source,
        &["padding", "opacity", "animation-duration", "width"],
    );
    assert_eq!(numeric_role(4, 7, "padding"), 7);
    assert_eq!(numeric_role(4, 7, "width"), 7);
    assert_eq!(numeric_role(4, 7, "opacity"), 263);
    assert_eq!(numeric_role(4, 7, "animation-duration"), 519);
    assert_eq!(numeric_role(6, 7, "opacity"), 7);
}

#[test]
fn single_unit_names_stay_unchanged_when_shorthand_targets_share_a_scale() {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = Expression::new_identifier(Span::new(4, 5), "n", &ast);
    let _scope = SiteScope::enter("source.tsx", "abcdefghijkl", &[]);
    plan_numeric_roles(&source, &["padding-left", "padding-right"]);
    assert_eq!(numeric_role(4, 2, "padding-left"), 2);
    assert_eq!(numeric_role(4, 2, "padding-right"), 2);
}

#[test]
fn numeric_roles_restore_when_nested_scope_and_folded_spans_change() {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = Expression::new_identifier(Span::new(4, 5), "n", &ast);
    let _outer = SiteScope::enter("outer.tsx", "abcdefghijkl", &[]);
    plan_numeric_roles(&source, &["padding", "opacity"]);
    retain_folded_owner(8, 4);
    assert_eq!(numeric_role(8, 0, "opacity"), 256);
    {
        let _inner = SiteScope::enter("inner.tsx", "abcdefghijkl", &[]);
        plan_numeric_roles(&source, &["opacity", "padding"]);
        assert_eq!(numeric_role(4, 0, "opacity"), 0);
        assert_eq!(numeric_role(4, 0, "padding"), 256);
    }
    assert_eq!(numeric_role(4, 0, "opacity"), 256);
}

#[test]
fn numeric_roles_use_the_default_when_no_source_scope_is_installed() {
    let allocator = Allocator::default();
    let ast = AstBuilder::new(&allocator);
    let source = Expression::new_identifier(Span::new(4, 5), "n", &ast);
    plan_numeric_roles(&source, &["padding", "opacity"]);
    assert_eq!(numeric_role(4, 2, "opacity"), 2);
}
