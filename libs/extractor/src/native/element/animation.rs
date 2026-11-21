//! Animations and transitions on React Native: react-native-reanimated runs
//! the `animation*` and `transition*` styles of its animated components, so an
//! element taking one renders Reanimated's, and `keyframes()` compiles to the
//! keyframes it reads.

use std::fmt::Write;

use oxc_ast::ast::{
    CallExpression, Expression, IdentifierReference, ObjectExpression, ObjectProperty,
    ObjectPropertyKind,
};
use oxc_span::GetSpan;

use super::Elements;
use crate::component::ExportVariableKind;
use crate::fresh_name::fresh_name;
use crate::native::REANIMATED;
use crate::native::errors::{Problem, reanimated_problem};
use crate::native::imports::native_component;
use crate::native::style::Choice;

/// The name `keyframes()` errors call the call by
const KEYFRAMES: &str = "keyframes()";

/// The React Native components Reanimated exports animated
const ANIMATED: [&str; 3] = ["View", "Text", "Image"];

/// Whether the style prop `name` animates: an `animation*` or `transition*`
/// prop
pub(super) fn is_animation(name: &str) -> bool {
    name.starts_with("animation") || name.starts_with("transition")
}

/// Reanimated's default export, and the components the file creates animated
pub(super) struct Animated {
    /// The name Reanimated's default export is imported under
    name: String,
    used: bool,
    /// Each component created animated: its local name and the animated
    /// component's
    created: Vec<(String, String)>,
}

impl Animated {
    pub(super) fn new(code: &str) -> Self {
        Self {
            name: fresh_name("__Animated", code),
            used: false,
            created: Vec::new(),
        }
    }

    /// The import of Reanimated, and the declaration of each component
    /// created animated
    pub(super) fn prelude(&self) -> String {
        if !self.used {
            return String::new();
        }
        let mut prelude = format!("import {} from \"{REANIMATED}\";\n", self.name);
        for (local, animated) in &self.created {
            let _ = writeln!(
                prelude,
                "const {animated} = {}.createAnimatedComponent({local});",
                self.name
            );
        }
        prelude
    }
}

impl<'a> Elements<'_> {
    /// Note the style prop `name` read at `offset` when it animates
    pub(super) fn note_animation(&mut self, name: &str, offset: u32) {
        if is_animation(name) {
            self.animation
                .get_or_insert_with(|| (offset, name.to_string()));
        }
    }

    /// Whether the app has react-native-reanimated, which the animation
    /// `name` at `offset` of `on` runs on; else a build error
    pub(super) fn reanimated(&mut self, (offset, name): &(u32, String), on: &str) -> bool {
        if !self.target.reanimated {
            self.error(*offset, reanimated_problem().on(on, name));
        }
        self.target.reanimated
    }

    /// The component an element of `kind` bound to `local` renders when it
    /// takes an animation: Reanimated's animated component, or `tag`, the
    /// component `as` gives
    pub(super) fn animated(
        &mut self,
        animation: &(u32, String),
        (kind, local): (&ExportVariableKind, &str),
        on: &str,
        tag: Option<String>,
        interactive: bool,
    ) -> Option<String> {
        if tag.is_some() && !interactive {
            // The component `as` gives animates when it is animated
            self.reanimated(animation, on);
            return tag;
        }
        self.animated_native(animation, (native_component(kind), local), on, interactive)
            .or(tag)
    }

    /// Reanimated's animated component of the React Native component
    /// `native`, which `local` is bound to, for an element taking an
    /// animation
    pub(super) fn animated_native(
        &mut self,
        animation: &(u32, String),
        (native, local): (&str, &str),
        on: &str,
        interactive: bool,
    ) -> Option<String> {
        if !self.reanimated(animation, on) {
            return None;
        }
        if interactive {
            self.error(
                animation.0,
                Problem::new(
                    "react-native-reanimated animates a `style` object, and a press, hover or focus style makes the `style` a function",
                    "animate an element inside the pressable one",
                )
                .on(on, &animation.1),
            );
            return None;
        }
        Some(self.animated_component(native, local))
    }

    /// Reanimated's animated component of the React Native component
    /// `native`, which `local` is bound to
    fn animated_component(&mut self, native: &str, local: &str) -> String {
        self.animated.used = true;
        if ANIMATED.contains(&native) {
            return format!("{}.{native}", self.animated.name);
        }
        if let Some((_, name)) = self.animated.created.iter().find(|(of, _)| of == local) {
            return name.clone();
        }
        let name = fresh_name(
            &format!("__Animated{}", local.trim_start_matches('_')),
            self.code,
        );
        self.animated
            .created
            .push((local.to_string(), name.clone()));
        name
    }

    /// Whether `callee` reads the `keyframes` the file imports
    pub(super) fn is_keyframes(&self, callee: &Expression<'_>) -> bool {
        match callee.without_parentheses() {
            Expression::Identifier(identifier) => self.is_keyframes_reference(identifier),
            _ => false,
        }
    }

    /// Whether `identifier` reads the `keyframes` the file imports
    pub(super) fn is_keyframes_reference(&self, identifier: &IdentifierReference<'_>) -> bool {
        identifier
            .reference_id
            .get()
            .and_then(|reference| self.scoping.get_reference(reference).symbol_id())
            .is_some_and(|symbol| self.keyframes.contains(&symbol))
    }

    /// Compile a call of `keyframes` to the keyframes Reanimated reads: an
    /// object of each keyframe's styles by its offset
    pub(super) fn keyframes_call(&mut self, call: &CallExpression<'a>) {
        let Some(object) = keyframes_object(call) else {
            self.error(
                call.span.start,
                Problem::new(
                    "the build reads the keyframes written in the call",
                    "pass one object of keyframes, such as `keyframes({ from: { opacity: 0 }, to: { opacity: 1 } })`",
                )
                .call(KEYFRAMES),
            );
            return;
        };
        let mut frames = Vec::new();
        for property in &object.properties {
            match property {
                ObjectPropertyKind::ObjectProperty(property) => {
                    frames.extend(self.keyframe(property));
                }
                ObjectPropertyKind::SpreadProperty(spread) => self.error(
                    spread.span.start,
                    Problem::new(
                        "the build reads only the keyframes written in the object",
                        "write each keyframe in it",
                    )
                    .call(KEYFRAMES),
                ),
            }
        }
        self.replacements.push((
            call.span.start as usize,
            call.span.end as usize,
            format!("{{ {} }}", frames.join(", ")),
        ));
    }

    /// The code of one keyframe: its offset and its styles
    fn keyframe(&mut self, property: &ObjectProperty<'a>) -> Option<String> {
        let key = property.key.span().source_text(self.code);
        let key = if property.computed {
            format!("[{key}]")
        } else {
            key.to_string()
        };
        let Expression::ObjectExpression(styles) = property.value.without_parentheses() else {
            self.error(
                property.value.span().start,
                Problem::new(
                    "a keyframe holds style props",
                    "give it an object, such as `{ opacity: 0 }`",
                )
                .on(KEYFRAMES, &key),
            );
            return None;
        };
        // A keyframe is no element: what it notes is no element's
        let outer = (self.press.take(), self.animation.take());
        let choices = self.object_choices(styles, KEYFRAMES);
        (self.press, self.animation) = outer;
        let mut values = Vec::new();
        if choices
            .into_iter()
            .all(|choice| frame_values(choice, &mut values))
        {
            let values: Vec<String> = values
                .iter()
                .map(|(key, value)| format!("{key}: {value}"))
                .collect();
            return Some(format!("{key}: {{ {} }}", values.join(", ")));
        }
        self.error(
            property.span.start,
            Problem::new(
                "react-native-reanimated reads each keyframe once, so its styles cannot differ by theme, breakpoint, orientation, state or condition",
                "write one value for each style of the keyframe",
            )
            .on(KEYFRAMES, &key),
        );
        None
    }
}

/// The object `keyframes()` is called with
fn keyframes_object<'b, 'a>(call: &'b CallExpression<'a>) -> Option<&'b ObjectExpression<'a>> {
    match call.arguments.as_slice() {
        [argument] => match argument.as_expression()?.without_parentheses() {
            Expression::ObjectExpression(object) => Some(object),
            _ => None,
        },
        _ => None,
    }
}

/// Add the styles of `choice` to `values`, each key with the code of its
/// value, a later key winning; `false` for a choice that differs by what the
/// runtime or a condition tells
fn frame_values(choice: Choice, values: &mut Vec<(String, String)>) -> bool {
    let declared: Vec<(String, String)> = match choice {
        Choice::Empty => Vec::new(),
        Choice::Static(declarations) => declarations
            .into_iter()
            .map(|(key, value)| (key, value.code()))
            .collect(),
        Choice::Dynamic(declared) => declared,
        Choice::All(choices) => {
            return choices
                .into_iter()
                .all(|choice| frame_values(choice, values));
        }
        _ => return false,
    };
    for (key, value) in declared {
        match values.iter_mut().find(|(name, _)| *name == key) {
            Some(existing) => existing.1 = value,
            None => values.push((key, value)),
        }
    }
    true
}
