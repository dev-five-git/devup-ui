use oxc_allocator::Allocator;
use oxc_ast::ast::CallExpression;
use oxc_ast_visit::{Visit, walk};
use oxc_parser::Parser;
use oxc_semantic::{Scoping, SemanticBuilder};
use oxc_span::SourceType;

use super::{StylexBindings, symbol};
use crate::stylex::StylexFunction;

fn calls(code: &str, package: &str) -> Vec<Option<StylexFunction>> {
    struct Calls<'s> {
        bindings: &'s StylexBindings,
        scoping: &'s Scoping,
        found: Vec<Option<StylexFunction>>,
    }
    impl<'a> Visit<'a> for Calls<'_> {
        fn visit_call_expression(&mut self, call: &CallExpression<'a>) {
            if super::super::stylex_sources::require_source(call).is_none() {
                self.found.push(
                    self.bindings
                        .function(&call.callee, &|id| symbol(self.scoping, id)),
                );
            }
            walk::walk_call_expression(self, call);
        }
    }
    let allocator = Allocator::default();
    let program = Parser::new(&allocator, code, SourceType::ts())
        .parse()
        .program;
    let scoping = SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    let bindings = StylexBindings::collect(&program, &scoping, package);
    let mut calls = Calls {
        bindings: &bindings,
        scoping: &scoping,
        found: Vec::new(),
    };
    calls.visit_program(&program);
    calls.found
}

#[test]
fn entrypoints_resolve_when_their_actual_bindings_are_read() {
    let sources = [
        "import sx from '@stylexjs/stylex'; sx.positionTry({});",
        "import * as sx from '@stylexjs/stylex'; sx.positionTry({});",
        "import * as sx from '@stylexjs/stylex'; sx.default.positionTry({});",
        "import { positionTry as pt } from '@stylexjs/stylex'; pt({});",
        "import * as sx from '@custom/ui/stylex'; sx.positionTry({});",
        "import { positionTry as pt } from '@custom/ui/stylex'; pt({});",
        "import { stylex as sx } from '@custom/ui'; sx.positionTry({});",
        "import * as devup from '@custom/ui'; devup.stylex.positionTry({});",
        "const sx = require('@custom/ui/stylex'); sx.positionTry({});",
        "const { positionTry: pt } = require('@custom/ui/stylex'); pt({});",
        "const sx = require('@stylexjs/stylex'); sx.default.positionTry({});",
        "const { stylex: sx } = require('@custom/ui'); sx.positionTry({});",
        "const devup = require('@custom/ui'); devup.stylex.positionTry({});",
        "import * as devup from '@custom/ui'; const sx = devup.stylex; const pt = sx.positionTry; pt({});",
    ];
    for code in sources {
        let found = calls(code, "@custom/ui");
        assert_eq!(found, vec![Some(StylexFunction::PositionTry)], "{code}");
    }
}

#[test]
fn unrelated_bindings_are_preserved_when_names_or_sources_resemble_apis() {
    let code = "import * as sx from '@stylexjs/stylex';
import * as devup from '@custom/ui';
import invalidDefault from '@custom/ui/stylex';
import * as other from 'stylex';
import * as prefix from '@custom/ui-other';
function f(sx, devup, require) {
  const cjs = require('@custom/ui/stylex');
  sx.positionTry({}); devup.stylex.positionTry({}); cjs.positionTry({});
}
invalidDefault.positionTry({}); other.positionTry({}); prefix.positionTry({});
devup.getTheme();";
    let found = calls(code, "@custom/ui");
    assert_eq!(found, vec![None; 7]);
}

#[test]
fn missing_reference_metadata_is_not_unbound_loader_proof() {
    let allocator = Allocator::default();
    let program = Parser::new(&allocator, "require('@stylexjs/stylex');", SourceType::ts())
        .parse()
        .program;
    let scoping = SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    let oxc_ast::ast::Statement::ExpressionStatement(statement) = &program.body[0] else {
        panic!("expression fixture");
    };
    let oxc_ast::ast::Expression::CallExpression(call) = &statement.expression else {
        panic!("call fixture");
    };
    let oxc_ast::ast::Expression::Identifier(loader) = &call.callee else {
        panic!("loader fixture");
    };
    loader.reference_id.set(None);
    let proven = super::unbound_reference(&scoping, loader);
    assert!(!proven);
}
