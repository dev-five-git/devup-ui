use oxc_ast::ast::{AssignmentTarget, AssignmentTargetMaybeDefault, AssignmentTargetProperty};
use oxc_ast_visit::Visit;

use super::traversal::Collector;

impl Collector {
    pub fn assignment_pattern(&mut self, pattern: &AssignmentTarget<'_>) {
        match pattern {
            AssignmentTarget::ArrayAssignmentTarget(array) => {
                for element in array.elements.iter().flatten() {
                    self.assignment_default(element);
                }
                if let Some(rest) = &array.rest {
                    self.assignment_pattern(&rest.target);
                }
            }
            AssignmentTarget::ObjectAssignmentTarget(object) => {
                for property in &object.properties {
                    match property {
                        AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(property) => {
                            if let Some(init) = &property.init {
                                self.visit_expression(init);
                            }
                        }
                        AssignmentTargetProperty::AssignmentTargetPropertyProperty(property) => {
                            if property.computed
                                && let Some(key) = property.name.as_expression()
                            {
                                self.visit_expression(key);
                            }
                            self.assignment_default(&property.binding);
                        }
                    }
                }
                if let Some(rest) = &object.rest {
                    self.assignment_pattern(&rest.target);
                }
            }
            _ => {}
        }
    }

    fn assignment_default(&mut self, target: &AssignmentTargetMaybeDefault<'_>) {
        match target {
            AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(default) => {
                self.assignment_pattern(&default.binding);
                self.visit_expression(&default.init);
            }
            _ => {
                if let Some(target) = target.as_assignment_target() {
                    self.assignment_pattern(target);
                }
            }
        }
    }
}
