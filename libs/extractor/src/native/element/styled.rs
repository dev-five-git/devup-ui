//! `styled()` on React Native: a call compiles to a function component that
//! renders its target with the styles it describes, before the `style` it is
//! given.

use oxc_ast::ast::{CallExpression, Expression, IdentifierReference, TaggedTemplateExpression};
use oxc_span::{GetSpan, Span};

use super::{Elements, Frame, HookAt, component_name, defaults};
use crate::component::ExportVariableKind;
use crate::native::errors::Problem;
use crate::native::style::{Choice, INTERACTIONS, Reads, Vars, interaction_keys, style_code};

/// The name `styled()` errors call the call by
const STYLED: &str = "styled()";

/// What a `styled()` component renders
pub(super) enum Target {
    /// A Devup UI component, which renders its React Native component with
    /// the styles it declares first
    Devup(ExportVariableKind, String),
    /// Any other component, by its code
    Other(String),
}

impl<'a> Elements<'_> {
    /// Whether `callee` reads the `styled` the file imports
    pub(super) fn is_styled(&self, callee: &Expression<'_>) -> bool {
        match callee.without_parentheses() {
            Expression::Identifier(identifier) => self.is_styled_reference(identifier),
            _ => false,
        }
    }

    /// Whether `identifier` reads the `styled` the file imports
    pub(super) fn is_styled_reference(&self, identifier: &IdentifierReference<'_>) -> bool {
        identifier
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
            .is_some_and(|symbol| self.styled.contains(&symbol))
    }

    /// The target of `callee` when it is `styled(Target)`, whose call or
    /// tagged template gives the styles
    pub(super) fn styled_target(&mut self, callee: &Expression<'a>) -> Option<Target> {
        let Expression::CallExpression(call) = callee.without_parentheses() else {
            return None;
        };
        if !self.is_styled(&call.callee) || call.arguments.len() != 1 {
            return None;
        }
        let target = call.arguments[0].as_expression()?;
        Some(self.target(target))
    }

    /// The target `expression` names
    fn target(&mut self, expression: &Expression<'a>) -> Target {
        if let Expression::Identifier(identifier) = expression.without_parentheses()
            && let Some((kind, local)) = self.binding(identifier)
        {
            return Target::Devup(kind, local);
        }
        let code = expression.span().source_text(self.code).to_string();
        if !component_name(expression.without_parentheses()) {
            self.error(
                expression.span().start,
                Problem::new(
                    "a `styled()` component on React Native renders a component, not a tag",
                    "pass a Devup UI or React Native component, such as `styled(View)`",
                )
                .call(STYLED),
            );
        }
        Target::Other(code)
    }

    /// `styled(Target)(...styles)`
    pub(super) fn styled_call(&mut self, call: &CallExpression<'a>, target: Target) {
        let arguments: Vec<&Expression<'a>> = call
            .arguments
            .iter()
            .filter_map(|argument| argument.as_expression())
            .collect();
        if arguments.len() < call.arguments.len() {
            self.error(
                call.span.start,
                Problem::new(
                    "the build reads only the styles written in the call",
                    "pass each part to `styled()` itself",
                )
                .call(STYLED),
            );
        }
        self.styled_component(call.span, target, |visitor, choices| {
            for argument in arguments {
                visitor.css_part(argument, choices);
            }
        });
    }

    /// `` styled(Target)`css text` ``
    pub(super) fn styled_template(
        &mut self,
        template: &TaggedTemplateExpression<'a>,
        target: Target,
    ) {
        self.styled_component(template.span, target, |visitor, choices| {
            let text = visitor.css_text(&template.quasi, template.span.start);
            choices.extend(text);
        });
    }

    /// `styled(Target, styles)`, or `styled(Target)` alone, which gives no
    /// component until it is called with styles
    pub(super) fn styled_direct(&mut self, call: &CallExpression<'a>) {
        let parts = match call.arguments.as_slice() {
            [target, styles] => target.as_expression().zip(styles.as_expression()),
            _ => None,
        };
        let Some((target, styles)) = parts else {
            self.error(
                call.span.start,
                Problem::new(
                    "the build compiles `styled()` called with a component and its styles",
                    "write `styled(Target)({ ... })`, `` styled(Target)`...` `` or `styled(Target, { ... })`",
                )
                .call(STYLED),
            );
            return;
        };
        let target = self.target(target);
        self.styled_component(call.span, target, |visitor, choices| {
            visitor.css_part(styles, choices);
        });
    }

    /// Put the component rendering `target` with the styles `read` reads in
    /// place of the `styled()` at `span`
    fn styled_component(
        &mut self,
        span: Span,
        target: Target,
        read: impl FnOnce(&mut Self, &mut Vec<Choice>),
    ) {
        let (tag, mut choices) = match &target {
            Target::Devup(kind, local) => (local.clone(), defaults(kind)),
            Target::Other(code) => (code.clone(), Vec::new()),
        };
        // The component calls the hooks its styles read, as a function
        // component does
        self.frames.push(Frame {
            component: true,
            reads: Reads::default(),
            hook_at: HookAt::Body(span.start as usize),
        });
        let outer = std::mem::replace(&mut self.in_styled, true);
        read(self, &mut choices);
        self.in_styled = outer;
        let frame_reads = self
            .frames
            .pop()
            .map_or_else(Reads::default, |frame| frame.reads);
        let tag = match (self.press.take(), &target) {
            (Some(press), Target::Devup(kind, _)) => {
                self.pressable(press, kind, "styled()", None).unwrap_or(tag)
            }
            (Some((offset, name)), Target::Other(_)) => {
                self.error(
                    offset,
                    Problem::new(
                        "a press, hover or focus style needs React Native's `Pressable`, which the build knows only a Devup UI component renders",
                        "style a `Box` or a `Button`, or pass a function reading `pressed` to the `style` of a `Pressable`",
                    )
                    .on(STYLED, &name),
                );
                tag
            }
            (None, _) => tag,
        };
        let tag = match (self.animation.take(), &target) {
            (Some(animation), Target::Devup(kind, local)) => {
                let interactive = INTERACTIONS
                    .iter()
                    .any(|state| choices.iter().any(|choice| choice.holds(*state)));
                self.animated(&animation, (kind, local), STYLED, None, interactive)
                    .unwrap_or(tag)
            }
            (Some(animation), Target::Other(_)) => {
                // The component given animates when it is animated
                self.reanimated(&animation, STYLED);
                tag
            }
            (None, _) => tag,
        };
        let props = self.props.clone();
        let given = format!("{props}.style");
        let disabled = format!("{props}.disabled");
        let vars = Vars {
            disabled: &disabled,
            ..self.runtime.vars()
        };
        let keys = interaction_keys(&choices, &vars);
        let style = style_code(choices, (Vec::new(), vec![given]), &mut self.sheet, &vars);
        let reads = style
            .as_ref()
            .map_or_else(Reads::default, |style| style.reads)
            .union(frame_reads);
        let hooks = if reads.any() {
            format!("{} ", self.hook_calls(reads))
        } else {
            String::new()
        };
        let style = style.map_or_else(String::new, |style| {
            let code = if keys.is_empty() {
                style.code
            } else {
                format!("({{ {} }}) => {}", keys.join(", "), style.code)
            };
            format!(" style={{{code}}}")
        });
        let code = format!("({props}) => {{ {hooks}return <{tag} {{...{props}}}{style} />; }}");
        self.replacements
            .push((span.start as usize, span.end as usize, code));
    }
}

/// Whether `expression` is a function, which a `styled()` component calls
/// with its props
pub(super) fn is_function(expression: &Expression<'_>) -> bool {
    matches!(
        expression.without_parentheses(),
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
    )
}

/// Whether the function `expression` gives an object of style props, which
/// React Native cannot read: an object, or a condition choosing one
pub(super) fn gives_style_props(expression: &Expression<'_>) -> bool {
    let Expression::ArrowFunctionExpression(arrow) = expression.without_parentheses() else {
        return false;
    };
    arrow.get_expression().is_some_and(holds_object)
}

fn holds_object(expression: &Expression<'_>) -> bool {
    match expression.without_parentheses() {
        Expression::ObjectExpression(_) => true,
        Expression::ConditionalExpression(condition) => {
            holds_object(&condition.consequent) || holds_object(&condition.alternate)
        }
        Expression::LogicalExpression(logical) => holds_object(&logical.right),
        _ => false,
    }
}
