//! The props a styled component's style functions and attrs read: the
//! component keeps those away from the tag it renders unless the tag takes
//! them as attributes, and an element using the component keeps passing them

use oxc_ast::ast::{
    ArrowFunctionExpression, BindingPattern, Expression, FormalParameters, Function,
    IdentifierReference, ObjectPropertyKind, StaticMemberExpression,
};
use oxc_ast_visit::{Visit, walk};

use crate::utils::unwrap_syntax_only;

/// The props read by name, and whether some function reads the props whole,
/// so which props it needs only the runtime tells
#[derive(Default, Clone, Debug, PartialEq, Eq)]
pub struct Reads {
    pub names: Vec<String>,
    pub whole: bool,
}

impl Reads {
    fn push(&mut self, name: &str) {
        if !self.names.iter().any(|existing| existing == name) {
            self.names.push(name.to_string());
        }
    }

    /// The functions `expression` holds where a style or attrs reads props:
    /// the expression itself, or a value of a rule object
    pub fn read_in(&mut self, expression: &Expression<'_>) {
        match unwrap_syntax_only(expression) {
            Expression::ArrowFunctionExpression(arrow) => self.function(&arrow.params, |v| {
                v.visit_arrow_function_body(arrow);
            }),
            Expression::FunctionExpression(function) => self.function(&function.params, |v| {
                v.visit_function_body_of(function);
            }),
            Expression::ObjectExpression(object) => {
                for property in &object.properties {
                    match property {
                        ObjectPropertyKind::ObjectProperty(property) => {
                            self.read_in(&property.value);
                        }
                        ObjectPropertyKind::SpreadProperty(spread) => {
                            self.read_in(&spread.argument);
                        }
                    }
                }
            }
            Expression::ArrayExpression(array) => {
                for element in &array.elements {
                    if let Some(element) = element.as_expression() {
                        self.read_in(element);
                    }
                }
            }
            _ => {}
        }
    }

    fn function(&mut self, params: &FormalParameters<'_>, body: impl FnOnce(&mut PropsUse<'_>)) {
        let Some(first) = params.items.first() else {
            return;
        };
        match &first.pattern {
            BindingPattern::BindingIdentifier(identifier) => {
                let mut uses = PropsUse {
                    name: identifier.name.as_str(),
                    reads: self,
                };
                body(&mut uses);
            }
            BindingPattern::ObjectPattern(object) => {
                for property in &object.properties {
                    match property.key.static_name() {
                        Some(name) if !property.computed => self.push(&name),
                        _ => self.whole = true,
                    }
                }
                if object.rest.is_some() {
                    self.whole = true;
                }
            }
            _ => self.whole = true,
        }
    }
}

struct PropsUse<'r> {
    name: &'r str,
    reads: &'r mut Reads,
}

impl PropsUse<'_> {
    fn visit_arrow_function_body(&mut self, arrow: &ArrowFunctionExpression<'_>) {
        match &arrow.body {
            oxc_ast::ast::ArrowFunctionBody::FunctionBody(body) => self.visit_function_body(body),
            body => {
                if let Some(expression) = body.as_expression() {
                    self.visit_expression(expression);
                }
            }
        }
    }

    fn visit_function_body_of(&mut self, function: &Function<'_>) {
        if let Some(body) = &function.body {
            self.visit_function_body(body);
        }
    }
}

impl<'a> Visit<'a> for PropsUse<'_> {
    fn visit_static_member_expression(&mut self, it: &StaticMemberExpression<'a>) {
        if let Expression::Identifier(object) = &it.object
            && object.name == self.name
        {
            self.reads.push(&it.property.name);
            return;
        }
        walk::walk_static_member_expression(self, it);
    }

    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        if it.name == self.name {
            self.reads.whole = true;
        }
    }
}

/// What `shouldForwardProp` decides for a prop name, as the build evaluates it
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Forward {
    Always(bool),
    Is(String),
    In(Vec<String>),
    StartsWith(String),
    /// A prop React passes to an element, as `isPropValid` or styled-components'
    /// default validator tells
    Valid,
    Not(Box<Forward>),
    And(Box<Forward>, Box<Forward>),
    Or(Box<Forward>, Box<Forward>),
}

impl Forward {
    /// The prop names the rule compares with by name
    fn names(&self, names: &mut Vec<String>) {
        match self {
            Forward::Is(prop) => names.push(prop.clone()),
            Forward::In(props) => names.extend(props.iter().cloned()),
            Forward::Not(rule) => rule.names(names),
            Forward::And(left, right) | Forward::Or(left, right) => {
                left.names(names);
                right.names(names);
            }
            Forward::Always(_) | Forward::StartsWith(_) | Forward::Valid => {}
        }
    }

    #[must_use]
    pub fn forwards(&self, name: &str) -> bool {
        match self {
            Forward::Always(value) => *value,
            Forward::Is(prop) => prop == name,
            Forward::In(props) => props.iter().any(|prop| prop == name),
            Forward::StartsWith(prefix) => name.starts_with(prefix.as_str()),
            Forward::Valid => crate::prop_valid::is_prop_valid(name),
            Forward::Not(rule) => !rule.forwards(name),
            Forward::And(left, right) => left.forwards(name) && right.forwards(name),
            Forward::Or(left, right) => left.forwards(name) || right.forwards(name),
        }
    }

    /// `shouldForwardProp` as the build evaluates it: a function of the prop
    /// name comparing it with strings, `includes`, `startsWith`, `isPropValid`
    /// or the validator styled-components passes, joined by `!`, `&&` and `||`
    #[must_use]
    pub fn read(expression: &Expression<'_>) -> Option<Self> {
        let (params, body) = match unwrap_syntax_only(expression) {
            Expression::ArrowFunctionExpression(arrow) => (
                &arrow.params,
                match &arrow.body {
                    oxc_ast::ast::ArrowFunctionBody::FunctionBody(body) => single_expression(body)?,
                    body => body.as_expression()?,
                },
            ),
            Expression::FunctionExpression(function) => (
                &function.params,
                single_expression(function.body.as_ref()?)?,
            ),
            _ => return None,
        };
        let name = |index: usize| match params.items.get(index).map(|p| &p.pattern) {
            Some(BindingPattern::BindingIdentifier(identifier)) => Some(identifier.name.as_str()),
            _ => None,
        };
        // Without a parameter no identifier reads the prop, so only literals decide
        rule(body, name(0).unwrap_or_default(), name(1))
    }
}

fn single_expression<'b, 'a>(
    body: &'b oxc_ast::ast::FunctionBody<'a>,
) -> Option<&'b Expression<'a>> {
    match body.statements.as_slice() {
        [oxc_ast::ast::Statement::ReturnStatement(statement)] => statement.argument.as_ref(),
        _ => None,
    }
}

fn rule(expression: &Expression<'_>, prop: &str, validator: Option<&str>) -> Option<Forward> {
    use oxc_ast::ast::{BinaryOperator, LogicalOperator, UnaryOperator};
    let is_prop = |e: &Expression<'_>| matches!(unwrap_syntax_only(e), Expression::Identifier(i) if i.name == prop);
    let string = |e: &Expression<'_>| match unwrap_syntax_only(e) {
        Expression::StringLiteral(literal) => Some(literal.value.to_string()),
        _ => None,
    };
    Some(match unwrap_syntax_only(expression) {
        Expression::BooleanLiteral(literal) => Forward::Always(literal.value),
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            Forward::Not(Box::new(rule(&unary.argument, prop, validator)?))
        }
        Expression::LogicalExpression(logical) => {
            let left = Box::new(rule(&logical.left, prop, validator)?);
            let right = Box::new(rule(&logical.right, prop, validator)?);
            match logical.operator {
                LogicalOperator::And => Forward::And(left, right),
                LogicalOperator::Or => Forward::Or(left, right),
                LogicalOperator::Coalesce => return None,
            }
        }
        Expression::BinaryExpression(binary) => {
            let value = if is_prop(&binary.left) {
                string(&binary.right)?
            } else if is_prop(&binary.right) {
                string(&binary.left)?
            } else {
                return None;
            };
            match binary.operator {
                BinaryOperator::StrictEquality | BinaryOperator::Equality => Forward::Is(value),
                BinaryOperator::StrictInequality | BinaryOperator::Inequality => {
                    Forward::Not(Box::new(Forward::Is(value)))
                }
                _ => return None,
            }
        }
        Expression::CallExpression(call) => {
            let [argument] = call.arguments.as_slice() else {
                return None;
            };
            let argument = argument.as_expression()?;
            match unwrap_syntax_only(&call.callee) {
                Expression::Identifier(callee)
                    if is_prop(argument)
                        && (Some(callee.name.as_str()) == validator
                            || callee.name == "isPropValid") =>
                {
                    Forward::Valid
                }
                Expression::StaticMemberExpression(member)
                    if member.property.name == "startsWith" && is_prop(&member.object) =>
                {
                    Forward::StartsWith(string(argument)?)
                }
                Expression::StaticMemberExpression(member)
                    if member.property.name == "includes" && is_prop(argument) =>
                {
                    let Expression::ArrayExpression(array) = unwrap_syntax_only(&member.object)
                    else {
                        return None;
                    };
                    Forward::In(
                        array
                            .elements
                            .iter()
                            .map(|element| string(element.as_expression()?))
                            .collect::<Option<_>>()?,
                    )
                }
                _ => return None,
            }
        }
        _ => return None,
    })
}

/// Whether a styled component passes the prop `name` on to what it renders:
/// a `$` prop never, otherwise as `forward` decides, or by default a tag only
/// what it takes as attributes and never `theme`, a component every prop
pub fn passes(name: &str, renders_tag: bool, forward: Option<&Forward>) -> bool {
    if name.starts_with('$') {
        return false;
    }
    match forward {
        Some(forward) => forward.forwards(name),
        None if renders_tag => name != "theme" && crate::prop_valid::is_prop_valid(name),
        None => true,
    }
}

/// The props a styled component reads and keeps away from what it renders,
/// and `theme` when a tag does not take it
pub fn withheld(reads: &Reads, renders_tag: bool, forward: Option<&Forward>) -> Vec<String> {
    let mut named = reads.names.clone();
    if let Some(forward) = forward {
        forward.names(&mut named);
    }
    let mut withheld: Vec<String> = Vec::new();
    for name in named {
        if !passes(&name, renders_tag, forward) && !withheld.contains(&name) {
            withheld.push(name);
        }
    }
    if renders_tag
        && !passes("theme", true, forward)
        && !withheld.iter().any(|name| name == "theme")
    {
        withheld.push("theme".to_string());
    }
    withheld
}
