//! Styles composed in one place, such as `css(base, cond && danger)`: when the
//! build knows the parts, a later part's declaration replaces an earlier one's
//! for the same property, selector, breakpoint and layer. Atomic classes alone
//! cannot express that, because which of two classes wins depends on their
//! order in the stylesheet, not on the order they were composed in.

use css::style_selector::StyleSelector;
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::ast::Expression;
use oxc_ast::builder::AstBuilder;

use crate::{ExtractStyleProp, ExtractStyleValue};

/// What a declaration competes on: property, selector, breakpoint and layer
#[derive(Clone, PartialEq, Eq)]
struct CascadeKey {
    property: String,
    selector: Option<StyleSelector>,
    level: u8,
    layer: Option<String>,
    order: u8,
}

impl CascadeKey {
    fn of(value: &ExtractStyleValue) -> Self {
        let (property, selector, level, layer) = match value {
            ExtractStyleValue::Static(style) => (
                style.property(),
                style.selector(),
                style.level(),
                style.layer(),
            ),
            ExtractStyleValue::Dynamic(style) => (
                style.property(),
                style.selector(),
                style.level(),
                style.layer(),
            ),
            // A preset: the only other value a style prop holds
            _ => ("typography", None, 0, None),
        };
        Self {
            property: property.to_string(),
            selector: selector.cloned(),
            level,
            layer: layer.map(ToString::to_string),
            order: match value {
                ExtractStyleValue::Static(style) => style.style_order().unwrap_or(255),
                ExtractStyleValue::Dynamic(style) => style.style_order().unwrap_or(255),
                _ => 255,
            },
        }
    }
}

/// What a key resolves to
enum Choice<'a> {
    Empty,
    Atom(ExtractStyleValue),
    Conditional {
        test: Expression<'a>,
        consequent: Box<Choice<'a>>,
        alternate: Box<Choice<'a>>,
    },
}

/// What a part does to a key
enum Overlay<'a> {
    /// Leaves what came before
    Keep,
    Atom(ExtractStyleValue),
    Conditional {
        test: Expression<'a>,
        consequent: Box<Overlay<'a>>,
        alternate: Box<Overlay<'a>>,
    },
}

/// The parts composed so far, in source order: a key a later part sets moves
/// to the end, as when merging objects
#[derive(Default)]
pub struct Composition<'a> {
    entries: Vec<(CascadeKey, Choice<'a>)>,
    /// Styles whose properties the build cannot pair up, kept as they are
    unkeyed: Vec<ExtractStyleProp<'a>>,
}

impl<'a> Composition<'a> {
    /// A part the build knows completely
    pub fn apply(&mut self, ast_builder: &AstBuilder<'a>, props: Vec<ExtractStyleProp<'a>>) {
        let mut overlays: Vec<(CascadeKey, Overlay<'a>)> = Vec::new();
        for prop in props {
            self.overlays(ast_builder, prop, &mut overlays);
        }
        for (key, overlay) in overlays {
            self.overlay(ast_builder, key, overlay);
        }
    }

    /// A part applying only while `test` holds, or `alternate` otherwise
    pub fn apply_conditional(
        &mut self,
        ast_builder: &AstBuilder<'a>,
        test: &Expression<'a>,
        consequent: Vec<ExtractStyleProp<'a>>,
        alternate: Vec<ExtractStyleProp<'a>>,
    ) {
        let prop = ExtractStyleProp::Conditional {
            condition: test.clone_in(ast_builder.allocator()),
            consequent: Some(Box::new(ExtractStyleProp::StaticArray(consequent))),
            alternate: Some(Box::new(ExtractStyleProp::StaticArray(alternate))),
        };
        self.apply(ast_builder, vec![prop]);
    }

    /// Each key a prop sets, with what it sets it to
    fn overlays(
        &mut self,
        ast_builder: &AstBuilder<'a>,
        prop: ExtractStyleProp<'a>,
        overlays: &mut Vec<(CascadeKey, Overlay<'a>)>,
    ) {
        match prop {
            ExtractStyleProp::Static(value) => {
                push_overlay(overlays, CascadeKey::of(&value), Overlay::Atom(value));
            }
            ExtractStyleProp::StaticArray(props) => {
                for prop in props {
                    self.overlays(ast_builder, prop, overlays);
                }
            }
            ExtractStyleProp::Enum { map, condition } => {
                let entries = map
                    .into_iter()
                    .map(|(key, props)| (key, ExtractStyleProp::StaticArray(props)))
                    .collect();
                self.overlays(
                    ast_builder,
                    key_choice(ast_builder, &condition, entries),
                    overlays,
                );
            }
            ExtractStyleProp::MemberExpression { map, expression } => {
                let entries = map.into_iter().map(|(key, prop)| (key, *prop)).collect();
                self.overlays(
                    ast_builder,
                    key_choice(ast_builder, &expression, entries),
                    overlays,
                );
            }
            ExtractStyleProp::Conditional {
                condition,
                consequent,
                alternate,
            } if [&consequent, &alternate]
                .into_iter()
                .flatten()
                .all(|side| keyed(side)) =>
            {
                let mut sides = [Vec::new(), Vec::new()];
                for (side, prop) in sides.iter_mut().zip([consequent, alternate]) {
                    if let Some(prop) = prop {
                        self.overlays(ast_builder, *prop, side);
                    }
                }
                let [mut consequent, mut alternate] = sides;
                let mut keys: Vec<CascadeKey> = Vec::new();
                for (key, _) in consequent.iter().chain(&alternate) {
                    if !keys.contains(key) {
                        keys.push(key.clone());
                    }
                }
                for key in keys {
                    let consequent = take_overlay(&mut consequent, &key);
                    let alternate = take_overlay(&mut alternate, &key);
                    push_overlay(
                        overlays,
                        key,
                        Overlay::Conditional {
                            test: condition.clone_in(ast_builder.allocator()),
                            consequent: Box::new(consequent),
                            alternate: Box::new(alternate),
                        },
                    );
                }
            }
            prop => self.unkeyed.push(prop),
        }
    }

    fn overlay(&mut self, ast_builder: &AstBuilder<'a>, key: CascadeKey, overlay: Overlay<'a>) {
        let previous = self
            .entries
            .iter()
            .position(|(existing, _)| *existing == key)
            .map_or(Choice::Empty, |index| self.entries.remove(index).1);
        let choice = resolve(ast_builder, overlay, previous);
        self.entries.push((key, choice));
    }

    /// A class applying after the composed parts, whose styles `values` the
    /// build knows: what it sets replaces what they set
    pub fn cover(&mut self, values: &[ExtractStyleValue]) {
        let keys: Vec<CascadeKey> = values.iter().map(CascadeKey::of).collect();
        self.entries.retain(|(key, _)| !keys.contains(key));
    }

    /// The composed styles, for class names and the stylesheet
    #[must_use]
    pub fn into_props(self) -> Vec<ExtractStyleProp<'a>> {
        self.entries
            .into_iter()
            .filter_map(|(_, choice)| into_prop(choice))
            .chain(self.unkeyed)
            .collect()
    }

    /// The composed atoms when no condition chooses between them
    #[must_use]
    pub fn unconditional(&self) -> Option<Vec<ExtractStyleValue>> {
        if !self.unkeyed.is_empty() {
            return None;
        }
        let mut values = Vec::new();
        for (_, choice) in &self.entries {
            values.push(fixed_choice(choice)?.clone());
        }
        Some(values)
    }
}

fn key_choice<'a>(
    ast: &AstBuilder<'a>,
    condition: &Expression<'a>,
    mut entries: Vec<(String, ExtractStyleProp<'a>)>,
) -> ExtractStyleProp<'a> {
    use oxc_allocator::FromIn;
    use oxc_ast::ast::{BinaryOperator, Str};
    use oxc_span::SPAN;
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    let mut previous = ExtractStyleProp::StaticArray(vec![]);
    for (key, prop) in entries.into_iter().rev() {
        previous = ExtractStyleProp::Conditional {
            condition: Expression::new_binary_expression(
                SPAN,
                condition.clone_in(ast.allocator()),
                BinaryOperator::StrictEquality,
                Expression::new_string_literal(
                    SPAN,
                    Str::from_in(key.as_str(), ast.allocator()),
                    None,
                    ast,
                ),
                ast,
            ),
            consequent: Some(Box::new(prop)),
            alternate: Some(Box::new(previous)),
        };
    }
    previous
}

fn fixed_choice<'s>(choice: &'s Choice<'_>) -> Option<&'s ExtractStyleValue> {
    match choice {
        Choice::Atom(value) => Some(value),
        Choice::Empty => None,
        Choice::Conditional {
            consequent,
            alternate,
            ..
        } => {
            let yes = fixed_choice(consequent)?;
            let no = fixed_choice(alternate)?;
            (yes == no).then_some(yes)
        }
    }
}

/// Styles a part of `css(...)` gives
pub enum KnownStyles<'a> {
    /// The styles of a `css()` class the file binds
    Known(Vec<ExtractStyleValue>),
    Finite(crate::finite_styles::FiniteStyles, Expression<'a>),
    /// A rule object
    Rules(Expression<'a>),
}

/// A side of a condition among the parts of `css(...)`
pub enum KnownSide<'a> {
    Styles(Vec<KnownStyles<'a>>),
    /// A class the build does not know the styles of
    Class(Expression<'a>),
    Empty,
}

/// A part of `css(...)`, in source order
pub enum KnownPart<'a> {
    Styles(Vec<KnownStyles<'a>>),
    Conditional {
        test: Expression<'a>,
        consequent: Vec<KnownStyles<'a>>,
        alternate: Vec<KnownStyles<'a>>,
    },
    Class(Expression<'a>),
}

/// `prop`'s styles at `order`, unless one sets its own
pub fn set_prop_order(prop: &mut ExtractStyleProp<'_>, order: u8) {
    match prop {
        ExtractStyleProp::Static(value) => value.set_style_order(order),
        ExtractStyleProp::StaticArray(props) => {
            for prop in props {
                set_prop_order(prop, order);
            }
        }
        ExtractStyleProp::Conditional {
            consequent,
            alternate,
            ..
        } => {
            for side in [consequent, alternate].into_iter().flatten() {
                set_prop_order(side, order);
            }
        }
        ExtractStyleProp::Enum { map, .. } => {
            for prop in map.values_mut().flatten() {
                set_prop_order(prop, order);
            }
        }
        ExtractStyleProp::MemberExpression { map, .. } => {
            for prop in map.values_mut() {
                set_prop_order(prop, order);
            }
        }
        // Class names the code gives, and styles reported as errors
        ExtractStyleProp::Expression { .. }
        | ExtractStyleProp::Unreadable { .. }
        | ExtractStyleProp::Diagnostic { .. } => {}
    }
}

/// Whether `later` sets a known key that `earlier` also sets.
pub fn overlaps(earlier: &[ExtractStyleProp<'_>], later: &[ExtractStyleProp<'_>]) -> bool {
    let earlier = keys(earlier);
    keys(later).iter().any(|key| earlier.contains(key))
}

/// Known keys `props` set; runtime classes have no known keys here.
fn keys(props: &[ExtractStyleProp<'_>]) -> Vec<CascadeKey> {
    let mut keys = Vec::new();
    for prop in props {
        match prop {
            ExtractStyleProp::Static(value) => keys.push(CascadeKey::of(value)),
            ExtractStyleProp::StaticArray(props) => keys.extend(self::keys(props)),
            ExtractStyleProp::Conditional {
                consequent,
                alternate,
                ..
            } => {
                for side in [consequent, alternate].into_iter().flatten() {
                    keys.extend(self::keys(std::slice::from_ref(side.as_ref())));
                }
            }
            ExtractStyleProp::Expression { .. }
            | ExtractStyleProp::Unreadable { .. }
            | ExtractStyleProp::Diagnostic { .. } => {}
            ExtractStyleProp::Enum { map, .. } => {
                for props in map.values() {
                    keys.extend(self::keys(props));
                }
            }
            ExtractStyleProp::MemberExpression { map, .. } => {
                for prop in map.values() {
                    keys.extend(self::keys(std::slice::from_ref(prop.as_ref())));
                }
            }
        }
    }
    keys
}

/// Whether every style `prop` holds has a key, so a condition around it can be
/// applied key by key
fn keyed(prop: &ExtractStyleProp<'_>) -> bool {
    match prop {
        ExtractStyleProp::Static(_) => true,
        ExtractStyleProp::StaticArray(props) => props.iter().all(keyed),
        ExtractStyleProp::Conditional {
            consequent,
            alternate,
            ..
        } => [consequent, alternate]
            .into_iter()
            .flatten()
            .all(|side| keyed(side)),
        ExtractStyleProp::Enum { map, .. } => map.values().flatten().all(keyed),
        ExtractStyleProp::MemberExpression { map, .. } => map.values().all(|prop| keyed(prop)),
        ExtractStyleProp::Expression { .. }
        | ExtractStyleProp::Unreadable { .. }
        | ExtractStyleProp::Diagnostic { .. } => false,
    }
}

/// `overlay` replacing what `overlays` already sets for the key: a part
/// declaring a property twice keeps the later declaration
fn push_overlay<'a>(
    overlays: &mut Vec<(CascadeKey, Overlay<'a>)>,
    key: CascadeKey,
    overlay: Overlay<'a>,
) {
    overlays.retain(|(existing, _)| *existing != key);
    overlays.push((key, overlay));
}

fn take_overlay<'a>(
    overlays: &mut Vec<(CascadeKey, Overlay<'a>)>,
    key: &CascadeKey,
) -> Overlay<'a> {
    overlays
        .iter()
        .position(|(existing, _)| existing == key)
        .map_or(Overlay::Keep, |index| overlays.remove(index).1)
}

/// What `overlay` makes of `previous`
fn resolve<'a>(
    ast_builder: &AstBuilder<'a>,
    overlay: Overlay<'a>,
    previous: Choice<'a>,
) -> Choice<'a> {
    match overlay {
        Overlay::Keep => previous,
        Overlay::Atom(value) => Choice::Atom(value),
        Overlay::Conditional {
            test,
            consequent,
            alternate,
        } => {
            let copy = copy_choice(ast_builder, &previous);
            Choice::Conditional {
                test,
                consequent: Box::new(resolve(ast_builder, *consequent, previous)),
                alternate: Box::new(resolve(ast_builder, *alternate, copy)),
            }
        }
    }
}

fn copy_choice<'a>(ast_builder: &AstBuilder<'a>, choice: &Choice<'a>) -> Choice<'a> {
    match choice {
        Choice::Empty => Choice::Empty,
        Choice::Atom(value) => Choice::Atom(value.clone()),
        Choice::Conditional {
            test,
            consequent,
            alternate,
        } => Choice::Conditional {
            test: test.clone_in(ast_builder.allocator()),
            consequent: Box::new(copy_choice(ast_builder, consequent)),
            alternate: Box::new(copy_choice(ast_builder, alternate)),
        },
    }
}

fn into_prop(choice: Choice<'_>) -> Option<ExtractStyleProp<'_>> {
    match choice {
        Choice::Empty => None,
        Choice::Atom(value) => Some(ExtractStyleProp::Static(value)),
        Choice::Conditional {
            test,
            consequent,
            alternate,
        } => {
            let (consequent, alternate) = (into_prop(*consequent), into_prop(*alternate));
            (consequent.is_some() || alternate.is_some()).then(|| ExtractStyleProp::Conditional {
                condition: test,
                consequent: consequent.map(Box::new),
                alternate: alternate.map(Box::new),
            })
        }
    }
}
