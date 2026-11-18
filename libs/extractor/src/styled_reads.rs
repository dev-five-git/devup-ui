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
                v.visit_formal_parameters(&arrow.params);
                v.visit_arrow_function_body(arrow);
            }),
            Expression::FunctionExpression(function) => {
                let mut arguments = ArgumentsUse(false);
                arguments.visit_formal_parameters(&function.params);
                if let Some(body) = &function.body {
                    arguments.visit_function_body(body);
                }
                self.whole |= arguments.0;
                self.function(&function.params, |v| {
                    v.visit_formal_parameters(&function.params);
                    v.visit_function_body_of(function);
                });
            }
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
            self.whole |= params.rest.is_some();
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
                        Some(name) if !property.computed => match name.as_str() {
                            Some(name) => self.push(name),
                            None => self.whole = true,
                        },
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

struct ArgumentsUse(bool);

impl<'a> Visit<'a> for ArgumentsUse {
    fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
        self.0 |= it.name == "arguments";
    }

    fn visit_function(&mut self, _: &Function<'a>, _: oxc_syntax::scope::ScopeFlags) {}
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

/// The `@emotion/is-prop-valid` a compiled `shouldForwardProp` calls, which
/// the module imports under this name wherever the component renders
pub const IS_PROP_VALID: &str = "__devupIsPropValid";

/// Whether the compiled module calls [`IS_PROP_VALID`], so must import it
#[must_use]
pub fn calls_is_prop_valid(program: &oxc_ast::ast::Program<'_>) -> bool {
    struct Calls(bool);
    impl<'a> Visit<'a> for Calls {
        fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
            self.0 |= it.name == IS_PROP_VALID;
        }
    }
    let mut calls = Calls(false);
    calls.visit_program(program);
    calls.0
}

/// What `shouldForwardProp` decides for a prop name, as the build evaluates it
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Forward {
    Always(bool),
    Is(String),
    In(Vec<String>),
    StartsWith(String),
    /// A prop React passes to an element, as `isPropValid` tells: `true` when
    /// the predicate calls `isPropValid`, which the runtime imports from
    /// `@emotion/is-prop-valid`, `false` for the validator Devup UI's own
    /// `styled` passes, which the runtime lacks
    Valid(bool),
    /// Whether the element styled-components v6 renders, the second argument
    /// of its `shouldForwardProp`, is a tag (`typeof element === 'string'`)
    Tag,
    Not(Box<Forward>),
    And(Box<Forward>, Box<Forward>),
    Or(Box<Forward>, Box<Forward>),
}

/// What the second parameter of `shouldForwardProp` is: the element
/// styled-components v6 renders, or the validator Devup UI's own `styled`
/// passes, as styled-components v5 did
#[derive(Clone, Copy)]
enum Second<'s> {
    Element(Option<&'s str>),
    Validator(Option<&'s str>),
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
            Forward::Always(_) | Forward::StartsWith(_) | Forward::Valid(_) | Forward::Tag => {}
        }
    }

    /// Both rules, a prop passing only when each passes it: a rule `left`
    /// already holds, or one passing every prop, adds nothing
    #[must_use]
    pub fn and(left: Forward, right: Forward) -> Forward {
        fn conjuncts(rule: Forward, into: &mut Vec<Forward>) {
            match rule {
                Forward::And(left, right) => {
                    conjuncts(*left, into);
                    conjuncts(*right, into);
                }
                Forward::Always(true) => {}
                rule if into.contains(&rule) => {}
                rule => into.push(rule),
            }
        }
        let mut all = Vec::new();
        conjuncts(left, &mut all);
        conjuncts(right, &mut all);
        all.into_iter()
            .reduce(|left, right| Forward::And(Box::new(left), Box::new(right)))
            .unwrap_or(Forward::Always(true))
    }

    /// Whether the rule reads the element it renders, which `as` can change
    #[must_use]
    pub fn reads_element(&self) -> bool {
        match self {
            Forward::Tag => true,
            Forward::Not(rule) => rule.reads_element(),
            Forward::And(left, right) | Forward::Or(left, right) => {
                left.reads_element() || right.reads_element()
            }
            Forward::Always(_)
            | Forward::Is(_)
            | Forward::In(_)
            | Forward::StartsWith(_)
            | Forward::Valid(_) => false,
        }
    }

    /// Whether the rule passes the prop `name` on to an element that is a tag
    /// when `tag`
    #[must_use]
    pub fn forwards(&self, name: &str, tag: bool) -> bool {
        match self {
            Forward::Always(value) => *value,
            Forward::Is(prop) => prop == name,
            Forward::In(props) => props.iter().any(|prop| prop == name),
            Forward::StartsWith(prefix) => name.starts_with(prefix.as_str()),
            Forward::Valid(_) => crate::prop_valid::is_prop_valid(name),
            Forward::Tag => tag,
            Forward::Not(rule) => !rule.forwards(name, tag),
            Forward::And(left, right) => left.forwards(name, tag) && right.forwards(name, tag),
            Forward::Or(left, right) => left.forwards(name, tag) || right.forwards(name, tag),
        }
    }

    /// The rule as code testing the prop name `key` and the element
    /// `element` at runtime, where every prop a spread brings meets it, or
    /// `None` when it calls a validator only the build has
    #[must_use]
    pub fn code(&self, key: &str, element: &str) -> Option<String> {
        let text = |value: &str| serde_json::Value::from(value).to_string();
        Some(match self {
            Forward::Always(value) => value.to_string(),
            Forward::Is(prop) => format!("{key} === {}", text(prop)),
            Forward::In(props) => format!(
                "[{}].includes({key})",
                props
                    .iter()
                    .map(|prop| text(prop))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Forward::StartsWith(prefix) => format!("{key}.startsWith({})", text(prefix)),
            Forward::Valid(true) => format!("{IS_PROP_VALID}({key})"),
            Forward::Valid(false) => return None,
            Forward::Tag => format!("typeof {element} === \"string\""),
            Forward::Not(rule) => match rule.as_ref() {
                Forward::Is(prop) => format!("{key} !== {}", text(prop)),
                rule => format!("!({})", rule.code(key, element)?),
            },
            Forward::And(left, right) => format!(
                "({}) && ({})",
                left.code(key, element)?,
                right.code(key, element)?
            ),
            Forward::Or(left, right) => format!(
                "({}) || ({})",
                left.code(key, element)?,
                right.code(key, element)?
            ),
        })
    }

    /// `shouldForwardProp` as the build evaluates it: a function of the prop
    /// name comparing it with strings, `includes`, `startsWith` or
    /// `isPropValid`, joined by `!`, `&&` and `||`. Its second parameter is
    /// the element styled-components v6 renders, which it can test with
    /// `typeof element === 'string'`, or elsewhere the validator Devup UI's
    /// own `styled` passes.
    #[must_use]
    pub fn read(expression: &Expression<'_>, styled_components: bool) -> Option<Self> {
        let (params, body) = predicate(expression)?;
        let name = |index: usize| match params.items.get(index).map(|p| &p.pattern) {
            Some(BindingPattern::BindingIdentifier(identifier)) => Some(identifier.name.as_str()),
            _ => None,
        };
        let second = if styled_components {
            Second::Element(name(1))
        } else {
            Second::Validator(name(1))
        };
        // Without a parameter no identifier reads the prop, so only literals decide
        rule(body, name(0).unwrap_or_default(), second)
    }

    /// Whether `expression`, a `shouldForwardProp` of styled-components v6,
    /// calls its second parameter as styled-components v5's validator, which
    /// v6 replaced with the element it renders
    #[must_use]
    pub fn calls_element(expression: &Expression<'_>) -> bool {
        let Some((params, body)) = predicate(expression) else {
            return false;
        };
        let Some(BindingPattern::BindingIdentifier(element)) =
            params.items.get(1).map(|p| &p.pattern)
        else {
            return false;
        };
        let mut calls = CallsName(element.name.as_str(), false);
        calls.visit_expression(body);
        calls.1
    }
}

/// Whether an expression calls the name
struct CallsName<'n>(&'n str, bool);

impl<'a> Visit<'a> for CallsName<'_> {
    fn visit_call_expression(&mut self, it: &oxc_ast::ast::CallExpression<'a>) {
        self.1 |= matches!(unwrap_syntax_only(&it.callee), Expression::Identifier(callee) if callee.name == self.0);
        walk::walk_call_expression(self, it);
    }
}

/// The parameters of a predicate and the one expression it returns
fn predicate<'b, 'a>(
    expression: &'b Expression<'a>,
) -> Option<(&'b FormalParameters<'a>, &'b Expression<'a>)> {
    match unwrap_syntax_only(expression) {
        Expression::ArrowFunctionExpression(arrow) => Some((
            &arrow.params,
            match &arrow.body {
                oxc_ast::ast::ArrowFunctionBody::FunctionBody(body) => single_expression(body)?,
                body => body.as_expression()?,
            },
        )),
        Expression::FunctionExpression(function) => Some((
            &function.params,
            single_expression(function.body.as_ref()?)?,
        )),
        _ => None,
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

/// `typeof element === 'string'` and its negations, testing whether the
/// element styled-components renders is a tag
fn tag_test(binary: &oxc_ast::ast::BinaryExpression<'_>, element: &str) -> Option<Forward> {
    use oxc_ast::ast::{BinaryOperator, UnaryOperator};
    let typeof_element = |e: &Expression<'_>| {
        matches!(unwrap_syntax_only(e), Expression::UnaryExpression(unary)
            if unary.operator == UnaryOperator::Typeof
                && matches!(unwrap_syntax_only(&unary.argument), Expression::Identifier(i) if i.name == element))
    };
    let string = |e: &Expression<'_>| matches!(unwrap_syntax_only(e), Expression::StringLiteral(literal) if literal.value == "string");
    if !(typeof_element(&binary.left) && string(&binary.right)
        || string(&binary.left) && typeof_element(&binary.right))
    {
        return None;
    }
    match binary.operator {
        BinaryOperator::StrictEquality | BinaryOperator::Equality => Some(Forward::Tag),
        BinaryOperator::StrictInequality | BinaryOperator::Inequality => {
            Some(Forward::Not(Box::new(Forward::Tag)))
        }
        _ => None,
    }
}

fn rule(expression: &Expression<'_>, prop: &str, second: Second<'_>) -> Option<Forward> {
    use oxc_ast::ast::{BinaryOperator, LogicalOperator, UnaryOperator};
    let is_prop = |e: &Expression<'_>| matches!(unwrap_syntax_only(e), Expression::Identifier(i) if i.name == prop);
    let string = |e: &Expression<'_>| match unwrap_syntax_only(e) {
        Expression::StringLiteral(literal) => literal.value.as_str().map(str::to_string),
        _ => None,
    };
    let (element, validator) = match second {
        Second::Element(element) => (element, None),
        Second::Validator(validator) => (None, validator),
    };
    Some(match unwrap_syntax_only(expression) {
        Expression::BooleanLiteral(literal) => Forward::Always(literal.value),
        Expression::UnaryExpression(unary) if unary.operator == UnaryOperator::LogicalNot => {
            Forward::Not(Box::new(rule(&unary.argument, prop, second)?))
        }
        Expression::LogicalExpression(logical) => {
            let left = Box::new(rule(&logical.left, prop, second)?);
            let right = Box::new(rule(&logical.right, prop, second)?);
            match logical.operator {
                LogicalOperator::And => Forward::And(left, right),
                LogicalOperator::Or => Forward::Or(left, right),
                LogicalOperator::Coalesce => return None,
            }
        }
        Expression::BinaryExpression(binary) => {
            if let Some(tag) = element.and_then(|element| tag_test(binary, element)) {
                return Some(tag);
            }
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
                    if is_prop(argument) && Some(callee.name.as_str()) == validator =>
                {
                    Forward::Valid(false)
                }
                Expression::Identifier(callee)
                    if is_prop(argument)
                        && callee.name == "isPropValid"
                        && Some(callee.name.as_str()) != element =>
                {
                    Forward::Valid(true)
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

// What a component styled-components v6 makes forwards: every prop but
/// `theme`, as it no longer validates props, unless `shouldForwardProp` also
/// decides (`$` props and `as` never reach it). Other libraries keep `forward`.
#[must_use]
pub fn library_forward(styled_components: bool, forward: Option<Forward>) -> Option<Forward> {
    if !styled_components {
        return forward;
    }
    let theme = Forward::Not(Box::new(Forward::Is("theme".to_string())));
    Some(match forward {
        Some(own) => Forward::and(theme, own),
        None => theme,
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
        // What the element decides is left to the runtime, which knows it
        Some(forward) => forward.forwards(name, renders_tag) || forward.reads_element(),
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
