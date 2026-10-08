use super::{
    Context, ELEMENT_METHODS, MUTATING_METHODS, Proof, ReturnedCapture, Use, binding_of, callees,
};
use oxc_ast::ast::{CallExpression, Expression};
use oxc_ast_visit::Visit;
use oxc_syntax::node::NodeId;

pub(super) struct CallSite<'s, 'a> {
    pub node: NodeId,
    pub expression: &'s CallExpression<'a>,
}

impl Context<'_, '_> {
    /// `path` read as a method called on what comes before its last key.
    pub(super) fn method_call(
        &self,
        call: CallSite<'_, '_>,
        at: u32,
        mut path: Vec<Option<String>>,
    ) -> Option<Use> {
        // A local call can return captured references into a new holder.
        let Some(method) = path.pop() else {
            let Expression::Identifier(identifier) = &call.expression.callee else {
                return None;
            };
            let symbol = binding_of(self.scoping, identifier)?;
            let proof = Proof {
                nodes: self.nodes,
                scoping: self.scoping,
            };
            let body = proof.factory(symbol)?;
            let mut captures = ReturnedCapture {
                proof: &proof,
                found: false,
            };
            captures.visit_expression(body);
            return captures.found.then(|| self.classify(call.node)).flatten();
        };
        if let Some(method) = method.as_deref() {
            if MUTATING_METHODS.contains(&method) {
                return Some(Use::Changes {
                    at,
                    depth: path.len() + 1,
                });
            }
            if ELEMENT_METHODS.contains(&method) {
                if !callees::pristine(
                    &Proof {
                        nodes: self.nodes,
                        scoping: self.scoping,
                    },
                    "Array",
                ) || !matches!(
                    self.init.map(|init| Proof {
                        nodes: self.nodes,
                        scoping: self.scoping
                    }
                    .expression(init)),
                    Some(crate::imported_constants::provenance::Shape::Array(_))
                ) {
                    return (!self.in_style(call.node)).then_some(Use::Calls { at, path });
                }
                path.push(None);
                return self.escapes(call.node, at, path);
            }
            if !self.uses_this(&path, method) {
                return None;
            }
        }
        (!self.in_style(call.node)).then_some(Use::Calls { at, path })
    }
}
