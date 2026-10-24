use super::styled_capability::StyledCapability;
use super::styled_css_slot::{StyledContext, StyledCssSlot};
#[path = "styled_retained.rs"]
mod retained;
#[path = "styled_retarget.rs"]
mod retarget;
pub use retarget::with_component;
#[path = "styled_residual.rs"]
mod residual;
#[path = "styled_stages.rs"]
mod stages;
use super::style_order_projection::{AuthoredStyles, extract_with_order};
use super::styled_jsx::StyledJsx;
use retained::Execution;
use rustc_hash::FxHashMap;

use crate::{
    ExtractStyleProp,
    component::ExportVariableKind,
    extract_style::extract_style_value::ExtractStyleValue,
    extractor::{
        ExtractResult,
        extract_style_from_expression::LiteralHandling,
        styled_record::prepare,
        styled_record_evaluation::{Evaluations, bind_render_body, visit_style_expressions},
    },
    gen_class_name::{gen_class_names, merge_expression_for_class_name},
    gen_style::gen_styles,
    styled_reads::{Forward, Reads, withheld as props_withheld},
    utils::{
        STYLE_OBJECT, build_time_error, call_with_values, expression_to_code,
        merge_object_expressions, readable_code, unreadable_styles, unwrap_syntax_only,
        unwrap_syntax_only_mut, wrap_array_filter, wrap_direct_call,
    },
};
use oxc_allocator::{CloneIn, FromIn, GetAllocator};
use oxc_ast::{
    ast::{
        Argument, BindingPattern, BindingProperty, BindingRestElement, CallExpression, Expression,
        FormalParameter, FormalParameterKind, FormalParameters, JSXAttributeItem, JSXAttributeName,
        JSXAttributeValue, JSXElementName, JSXOpeningElement, ObjectPropertyKind, PropertyKey, Str,
    },
    builder::AstBuilder,
};
use oxc_ast_visit::VisitMut;
use oxc_parser::{Parser, ParserReturn};
use oxc_span::{GetSpan, SPAN, SourceType};
use oxc_syntax::operator::{BinaryOperator, LogicalOperator, UnaryOperator};

#[path = "styled_props.rs"]
mod props;

#[path = "styled_scalar.rs"]
pub(super) mod scalar;

#[path = "styled_rule_choices.rs"]
mod rule_choices;

#[path = "styled_template_composition.rs"]
mod template_composition;

#[path = "styled_rule_parts.rs"]
mod rule_parts;

#[path = "styled_rule_records.rs"]
mod records;

#[path = "styled_template_bindings.rs"]
mod template_bindings;

#[path = "styled_arguments.rs"]
mod arguments;

#[cfg(test)]
#[path = "styled_render_body_tests.rs"]
mod render_body_tests;

/// The binding a styled component reads the component it renders through, when
/// only the runtime gives that component
const STYLED_BASE: &str = "DevupStyled";

/// The Devup UI component an expression reads, by the bindings of the file
pub type Kinds<'k> = &'k dyn Fn(&Expression<'_>) -> Option<ExportVariableKind>;

const STYLED_FACTORY: &str =
    "it renders a tag, a component or a value naming one, with rule objects or CSS text";

/// What a styled component renders: a tag or a component JSX names, with the
/// styles a Devup UI component brings, and the value `name` is bound to when
/// only the runtime gives the component
struct Base<'a> {
    name: String,
    target: Option<Expression<'a>>,
    styles: Option<Vec<ExtractStyleValue>>,
    bound: Option<Expression<'a>>,
}

/// A styled component the file defines, which a component extending it
/// renders in its place: what it renders, its classes and styles, and its
/// attrs, as the extension applies them before its own
pub struct StyledDefinition<'a> {
    name: String,
    target: Option<Expression<'a>>,
    bound: Option<Expression<'a>>,
    classes: Vec<Expression<'a>>,
    styles: Vec<ExtractStyleProp<'a>>,
    composition_styles: Option<AuthoredStyles<'a>>,
    attrs: Vec<Expression<'a>>,
    /// The props its styles and attrs read
    reads: Reads,
    /// Its `shouldForwardProp`, as the build evaluates it
    forward: Option<Forward>,
    /// The classes other styles select it by, which a component extending it
    /// gives too, as it renders this one
    markers: Vec<String>,
    inherited_markers: Vec<String>,
    bindings: Vec<(String, Expression<'a>)>,
    capability: Option<StyledCapability>,
    execution: Execution<'a>,
}

/// `marker` as a class among those a component gives
fn marker_class<'a>(ast_builder: &AstBuilder<'a>, marker: &str) -> Expression<'a> {
    Expression::new_string_literal(
        SPAN,
        Str::from_in(marker, ast_builder.allocator()),
        None,
        ast_builder,
    )
}

/// How the classes of a styled component are named: the file its atoms are
/// split into, and the classes other styles select it by
#[derive(Clone, Copy, Default)]
pub struct Naming<'s> {
    pub split_filename: Option<&'s str>,
    pub markers: &'s [String],
}

/// The bindings styled factories and their CSS mixins read in this module.
pub struct StyledBindings<'s> {
    pub imports: Kinds<'s>,
    pub values: &'s crate::style_values::StyleValues<'s>,
    pub inline_css: &'s FxHashMap<u32, Vec<ExtractStyleValue>>,
}

impl StyledBindings<'_> {
    fn styles(&self, expression: &Expression<'_>) -> Option<&[ExtractStyleValue]> {
        self.values
            .styles(unwrap_syntax_only(expression))
            .or_else(|| match unwrap_syntax_only(expression) {
                Expression::StringLiteral(literal) => {
                    self.inline_css.get(&literal.span.start).map(Vec::as_slice)
                }
                _ => None,
            })
    }
}

/// Whether a styled component renders a tag, which takes only valid
/// attributes, rather than a component
fn renders_tag(name: &str, bound: Option<&Expression<'_>>) -> bool {
    bound.is_none() && name.starts_with(|c: char| c.is_ascii_lowercase()) && !name.contains('.')
}

/// The shared legacy whitelist includes `on`, but a native tag cannot take
/// that styling flag as a boolean attribute. Explicit forwarding stays intact.
fn withheld(reads: &Reads, renders_tag: bool, forward: Option<&Forward>) -> Vec<String> {
    let mut names = props_withheld(reads, renders_tag, forward);
    if renders_tag && forward.is_none() && reads.names.iter().any(|name| name == "on") {
        names.push("on".to_string());
    }
    names
}

/// `own` applying after `inherited`: a prop passes only when both pass it
fn combine_forward(inherited: Option<&Forward>, own: Option<Forward>) -> Option<Forward> {
    match (inherited.cloned(), own) {
        (Some(inherited), Some(own)) => Some(Forward::And(Box::new(inherited), Box::new(own))),
        (inherited, own) => own.or(inherited),
    }
}

struct Rename<'b, 'a> {
    builder: &'b AstBuilder<'a>,
    names: FxHashMap<String, String>,
}

impl<'a> VisitMut<'a> for Rename<'_, 'a> {
    fn visit_identifier_reference(
        &mut self,
        identifier: &mut oxc_ast::ast::IdentifierReference<'a>,
    ) {
        let mut code = String::new();
        // Dynamic identifiers can hold emitted JS code. Captured names
        // were excluded from authored source, so only whole tokens change.
        let separator = |ch: char| !ch.is_alphanumeric() && ch != '_' && ch != '$';
        for chunk in identifier.name.split_inclusive(separator) {
            let token = chunk.trim_end_matches(separator);
            code.push_str(self.names.get(token).map_or(token, String::as_str));
            code.push_str(&chunk[token.len()..]);
        }
        identifier.name = oxc_ast::ast::Ident::from_in(code.as_str(), self.builder.allocator());
    }
}

impl<'a> StyledDefinition<'a> {
    fn visit_expressions(
        &mut self,
        builder: &AstBuilder<'a>,
        mut visit: impl FnMut(&mut Expression<'a>),
    ) {
        for expression in self
            .target
            .iter_mut()
            .chain(&mut self.bound)
            .chain(&mut self.classes)
            .chain(&mut self.attrs)
        {
            visit(expression);
        }
        for (_, value) in &mut self.bindings {
            visit(value);
        }
        for style in &mut self.styles {
            visit_style_expressions(builder, style, &mut visit);
        }
        if let Some(styles) = &mut self.composition_styles {
            for style in &mut styles.0 {
                visit_style_expressions(builder, style, &mut visit);
            }
        }
    }

    fn rebind(&self, builder: &AstBuilder<'a>, scope: &mut Expression<'a>) -> Self {
        if !self.execution.replays() && self.bindings.is_empty() {
            return self.clone_in(builder.allocator());
        }
        let allocator = builder.allocator();
        let mut inherited = Self {
            name: self.name.clone(),
            target: self.target.as_ref().map(|value| value.clone_in(allocator)),
            composition_styles: self
                .composition_styles
                .as_ref()
                .map(|styles| styles.clone_in(allocator)),
            bound: self.bound.as_ref().map(|value| value.clone_in(allocator)),
            classes: clone_all(builder, &self.classes).collect(),
            styles: self
                .styles
                .iter()
                .map(|style| style.clone_in(allocator))
                .collect(),
            attrs: clone_all(builder, &self.attrs).collect(),
            reads: self.reads.clone(),
            forward: self.forward.clone(),
            markers: self.markers.clone(),
            inherited_markers: self.inherited_markers.clone(),
            capability: self.capability.clone(),
            execution: self.execution.clone_in(allocator),
            bindings: self
                .bindings
                .iter()
                .map(|(name, value)| (name.clone(), value.clone_in(allocator)))
                .collect(),
        };
        let mut reserved = vec![scope.clone_in(allocator)];
        inherited.visit_expressions(builder, |value| reserved.push(value.clone_in(allocator)));
        reserved.extend(inherited.bindings.iter().map(|(name, _)| {
            Expression::new_identifier(
                SPAN,
                oxc_ast::ast::Ident::from_in(name.as_str(), allocator),
                builder,
            )
        }));
        let reservation = Expression::new_array_expression(
            SPAN,
            oxc_allocator::Vec::from_iter_in(reserved.into_iter().map(Into::into), builder),
            builder,
        );
        let mut reads = Evaluations::new(builder, readable_code(&reservation));
        let mut rename = Rename {
            builder,
            names: FxHashMap::default(),
        };
        for (name, _) in &mut inherited.bindings {
            let fresh = reads.fresh_name();
            rename.names.insert(name.clone(), fresh.clone());
            *name = fresh;
        }
        for (_, value) in &mut inherited.bindings {
            rename.visit_expression(value);
        }
        for class in &mut inherited.classes {
            rename.visit_expression(class);
        }
        for style in &mut inherited.styles {
            visit_style_expressions(builder, style, &mut |value| rename.visit_expression(value));
        }
        if let Some(styles) = &mut inherited.composition_styles {
            for style in &mut styles.0 {
                visit_style_expressions(builder, style, &mut |value| {
                    rename.visit_expression(value);
                });
            }
        }
        let mut reserved = vec![reservation];
        reserved.extend(inherited.bindings.iter().map(|(name, _)| {
            Expression::new_identifier(
                SPAN,
                oxc_ast::ast::Ident::from_in(name.as_str(), allocator),
                builder,
            )
        }));
        *scope = Expression::new_array_expression(
            SPAN,
            oxc_allocator::Vec::from_iter_in(reserved.into_iter().map(Into::into), builder),
            builder,
        );
        inherited
    }
}

impl StyledDefinition<'_> {
    /// Whether an element using this component keeps passing the prop `name`:
    /// what its styles read, what it passes on, and what React or the
    /// component itself takes
    #[must_use]
    pub fn takes(&self, name: &str) -> bool {
        self.reads.whole
            || self.reads.names.iter().any(|read| read == name)
            || matches!(
                name,
                "className" | "style" | "as" | "forwardedAs" | "key" | "ref" | "children"
            )
            || crate::styled_reads::passes(name, self.native(), self.forward.as_ref())
    }

    /// Whether an extension can render what this definition renders: a value
    /// only the runtime gives is read again only when it is a binding
    #[must_use]
    pub fn extendable(&self) -> bool {
        self.bound
            .as_ref()
            .is_none_or(|bound| matches!(bound, Expression::Identifier(_)))
    }
}

impl<'a> StyledDefinition<'a> {
    /// The tag an element using this component can render in its place, with
    /// the styles the component gives it: a tag, with no attrs, no props read
    /// and no value only the runtime gives
    #[must_use]
    pub fn inline(&self) -> Option<(&str, &[ExtractStyleProp<'a>])> {
        (self.fixed_native() && self.bindings.is_empty() && self.execution.replays())
            .then_some((self.name.as_str(), self.styles.as_slice()))
    }

    fn fixed_native(&self) -> bool {
        renders_tag(&self.name, self.bound.as_ref())
            && self.attrs.is_empty()
            && self.reads == Reads::default()
            && self.classes.is_empty()
            && self.styles.iter().all(fixed)
    }

    fn with_css_slot(mut self, slot: Option<&StyledCapability>, inherited: Option<&Self>) -> Self {
        self.capability = slot
            .filter(|_| {
                (!self.fixed_summary() && !self.portable())
                    || (self.fixed_native() && !self.bindings.is_empty())
                    || !self.execution.replays()
                    || (!self.native()
                        && self.fixed_summary()
                        && inherited.is_some_and(|base| {
                            base.bindings.is_empty()
                                && base.fixed_summary()
                                && (base.portable() || base.capability.is_some())
                        }))
            })
            .cloned();
        self
    }

    pub(crate) fn css_slot(&self) -> Option<&StyledCssSlot> {
        self.capability
            .as_ref()
            .filter(|_| self.fixed_summary())
            .map(|capability| &capability.classes)
    }

    /// The classes other styles select the component by
    #[must_use]
    pub fn markers(&self) -> &[String] {
        &self.markers
    }

    /// This definition in the arena of another file
    #[must_use]
    pub fn clone_in<'b>(&self, allocator: &'b oxc_allocator::Allocator) -> StyledDefinition<'b> {
        StyledDefinition {
            name: self.name.clone(),
            composition_styles: self
                .composition_styles
                .as_ref()
                .map(|styles| styles.clone_in(allocator)),
            target: self
                .target
                .as_ref()
                .map(|target| target.clone_in(allocator)),
            bound: self.bound.as_ref().map(|bound| bound.clone_in(allocator)),
            classes: self
                .classes
                .iter()
                .map(|class| class.clone_in(allocator))
                .collect(),
            styles: self
                .styles
                .iter()
                .map(|style| style.clone_in(allocator))
                .collect(),
            attrs: self
                .attrs
                .iter()
                .map(|attr| attr.clone_in(allocator))
                .collect(),
            reads: self.reads.clone(),
            forward: self.forward.clone(),
            markers: self.markers.clone(),
            inherited_markers: self.inherited_markers.clone(),
            bindings: self
                .bindings
                .iter()
                .map(|(name, value)| (name.clone(), value.clone_in(allocator)))
                .collect(),
            capability: self.capability.clone(),
            execution: self.execution.clone_in(allocator),
        }
    }

    /// Whether another file can render this component in its place: it
    /// renders a tag, and nothing it holds reads a binding of the file
    /// defining it
    #[must_use]
    pub fn portable(&self) -> bool {
        self.bindings.is_empty()
            && self.execution.replays()
            && renders_tag(&self.name, self.bound.as_ref())
            && closed(self.code())
    }

    /// The code the component evaluates for its classes, attrs and styles
    fn code(&self) -> impl Iterator<Item = String> + '_ {
        self.classes
            .iter()
            .chain(&self.attrs)
            .chain(self.styles.iter().flat_map(style_expressions))
            .map(expression_to_code)
            .chain(
                self.styles
                    .iter()
                    .flat_map(ExtractStyleProp::extract)
                    .filter_map(|style| match style {
                        ExtractStyleValue::Dynamic(style) => Some(style.identifier().to_string()),
                        _ => None,
                    }),
            )
    }

    fn with_markers(mut self, markers: &[String], inherited: Option<&Self>) -> Self {
        self.markers = markers.to_vec();
        self.inherited_markers = inherited
            .into_iter()
            .flat_map(|definition| {
                definition
                    .markers
                    .iter()
                    .chain(&definition.inherited_markers)
            })
            .cloned()
            .collect();
        self
    }

    pub(crate) fn marker_classes(&self, ast: &AstBuilder<'a>) -> Option<Expression<'a>> {
        merge_expression_for_class_name(
            ast,
            self.markers
                .iter()
                .chain(&self.inherited_markers)
                .map(|marker| marker_class(ast, marker)),
        )
    }

    /// The styles the component gives what it renders
    #[must_use]
    pub fn styles(&self) -> &[ExtractStyleProp<'a>] {
        &self.styles
    }

    pub(crate) fn composition_styles(&self) -> &[ExtractStyleProp<'a>] {
        self.composition_styles
            .as_ref()
            .map_or(&self.styles, |styles| &styles.0)
    }

    fn with_composition_styles(mut self, styles: Option<AuthoredStyles<'a>>) -> Self {
        self.composition_styles = styles;
        self
    }
}

/// Whether `prop` holds only values the build knows
fn fixed(prop: &ExtractStyleProp<'_>) -> bool {
    match prop {
        ExtractStyleProp::Static(value) => !matches!(value, ExtractStyleValue::Dynamic(_)),
        ExtractStyleProp::StaticArray(props) => props.iter().all(fixed),
        _ => false,
    }
}

/// What extracting a styled component gives
pub struct StyledExtraction<'a> {
    pub result: ExtractResult<'a>,
    pub expression: Expression<'a>,
    pub errors: Vec<(u32, String)>,
    pub definition: Option<StyledDefinition<'a>>,
}

/// The styles a component renders: those it extends or its base component
/// brings, then its own, a later declaration replacing an earlier one
fn compose_styles<'a>(
    ast_builder: &AstBuilder<'a>,
    inherited: Option<&StyledDefinition<'a>>,
    defaults: Option<Vec<ExtractStyleValue>>,
    mut own: Vec<ExtractStyleProp<'a>>,
    authored: Option<AuthoredStyles<'a>>,
) -> (Vec<ExtractStyleProp<'a>>, Option<AuthoredStyles<'a>>) {
    let mut earlier: Vec<ExtractStyleProp<'a>> = inherited.map_or_else(Vec::new, |inherited| {
        inherited
            .composition_styles()
            .iter()
            .map(|style| style.clone_in(ast_builder.allocator()))
            .collect()
    });
    earlier.extend(defaults.into_iter().flatten().map(ExtractStyleProp::Static));
    if earlier.is_empty() {
        let summary = authored.map(|styles| styles.normalized(ast_builder));
        if let Some(summary) = &summary {
            own.splice(
                0..0,
                summary
                    .0
                    .iter()
                    .filter(|prop| {
                        matches!(
                            prop,
                            ExtractStyleProp::Static(ExtractStyleValue::LayerOrder(_))
                        )
                    })
                    .map(|prop| prop.clone_in(ast_builder.allocator())),
            );
        }
        return (own, summary);
    }
    let mut composition = crate::composition::Composition::default();
    composition.apply(ast_builder, earlier);
    match authored {
        Some(authored) => authored.apply(ast_builder, &mut composition),
        None => composition.apply(ast_builder, own),
    }
    (composition.into_props(), None)
}

/// The classes and attrs a component extending `inherited` starts from
fn inherited_parts<'a>(
    ast_builder: &AstBuilder<'a>,
    inherited: Option<&StyledDefinition<'a>>,
    attrs: &[Expression<'a>],
) -> (Vec<Expression<'a>>, Vec<Expression<'a>>) {
    let allocator = ast_builder.allocator();
    let (classes, mut all_attrs) = inherited.map_or_else(
        || (Vec::new(), Vec::new()),
        |inherited| {
            (
                clone_all(ast_builder, &inherited.classes).collect(),
                inherited
                    .attrs
                    .iter()
                    .map(|a| a.clone_in_with_semantic_ids(allocator))
                    .collect(),
            )
        },
    );
    all_attrs.extend(
        attrs
            .iter()
            .map(|attr| attr.clone_in_with_semantic_ids(allocator)),
    );
    (classes, all_attrs)
}

impl<'a> Base<'a> {
    /// What `inherited` renders, in place of the component defining it
    fn extending(
        self,
        ast_builder: &AstBuilder<'a>,
        inherited: Option<&StyledDefinition<'a>>,
    ) -> Self {
        match inherited {
            Some(inherited) => Self {
                name: inherited.name.clone(),
                target: inherited
                    .target
                    .as_ref()
                    .map(|target| target.clone_in(ast_builder.allocator())),
                styles: None,
                bound: inherited
                    .bound
                    .as_ref()
                    .map(|bound| bound.clone_in(ast_builder.allocator())),
            },
            None => self,
        }
    }

    /// The definition of the component rendering this base
    fn definition(
        &self,
        ast_builder: &AstBuilder<'a>,
        classes: Vec<Expression<'a>>,
        styles: &[ExtractStyleProp<'a>],
        attrs: &[Expression<'a>],
        props: (Reads, Option<Forward>),
        bindings: &[(String, Expression<'a>)],
    ) -> StyledDefinition<'a> {
        let allocator = ast_builder.allocator();
        let (reads, forward) = props;
        StyledDefinition {
            name: self.name.clone(),
            target: self
                .target
                .as_ref()
                .map(|target| target.clone_in(allocator)),
            bound: self.bound.as_ref().map(|bound| bound.clone_in(allocator)),
            classes,
            composition_styles: None,
            styles: styles
                .iter()
                .map(|style| style.clone_in(allocator))
                .collect(),
            attrs: attrs
                .iter()
                .map(|attr| attr.clone_in_with_semantic_ids(allocator))
                .collect(),
            reads,
            forward,
            markers: Vec::new(),
            inherited_markers: Vec::new(),
            capability: None,
            execution: Execution::Replay,
            bindings: bindings
                .iter()
                .map(|(name, value)| (name.clone(), value.clone_in(allocator)))
                .collect(),
        }
    }

    const fn named(name: String) -> Self {
        Self {
            name,
            target: None,
            styles: None,
            bound: None,
        }
    }

    /// `component` with the base bound to the name it renders, when it is a
    /// runtime value
    fn render(self, ast_builder: &AstBuilder<'a>, component: Expression<'a>) -> Expression<'a> {
        let component = forward_ref(ast_builder, component);
        match self.bound {
            Some(bound) => call_with_values(ast_builder, vec![(self.name, bound)], component),
            None => component,
        }
    }
}

/// The component `styled(Component)` extends, as written: the argument naming
/// it in a tagged template `styled(Component)`, `styled(Component)({...})` or
/// `styled(Component, {...})`
#[must_use]
pub fn extended<'b, 'a>(expression: &'b Expression<'a>) -> Option<&'b Expression<'a>> {
    let factory = match expression {
        Expression::TaggedTemplateExpression(tag) => &tag.tag,
        Expression::CallExpression(call)
            if matches!(
                unwrap_syntax_only(&call.callee),
                Expression::CallExpression(_)
            ) =>
        {
            &call.callee
        }
        expression => expression,
    };
    let Expression::CallExpression(call) = unwrap_syntax_only(factory) else {
        return None;
    };
    unwrap_syntax_only(call.arguments.first()?.as_expression()?).into()
}

fn extract_base_tag_and_class_name<'a>(
    ast_builder: &AstBuilder<'a>,
    input: &Expression<'a>,
    imports: Kinds<'_>,
) -> Option<Base<'a>> {
    match unwrap_syntax_only(input) {
        Expression::StaticMemberExpression(member) => {
            Some(Base::named(member.property.name.to_string()))
        }
        // styled("div") or styled(Component)
        Expression::CallExpression(call) if call.arguments.len() == 1 => {
            tag_from_argument(ast_builder, &call.arguments[0], imports)
        }
        _ => None,
    }
}

/// Read the base out of a `styled(...)` argument: a devup-ui component gives its HTML
/// tag and the styles it contributes by default, a component or a member such as
/// `motion.div` is rendered as named, and any other value (a lowercase variable, a
/// call such as `forwardRef(...)`) is read at runtime.
fn tag_from_argument<'a>(
    ast_builder: &AstBuilder<'a>,
    argument: &Argument<'a>,
    imports: Kinds<'_>,
) -> Option<Base<'a>> {
    let argument = unwrap_syntax_only(argument.as_expression()?);
    match argument {
        Expression::StringLiteral(lit) => return Some(Base::named(lit.value.to_string())),
        // Never a component
        Expression::NullLiteral(_)
        | Expression::BooleanLiteral(_)
        | Expression::NumericLiteral(_) => return None,
        Expression::Identifier(ident) if ident.name == "undefined" => return None,
        _ => {}
    }
    if let Some(kind) = imports(argument) {
        return Some(Base {
            styles: Some(kind.extract()),
            ..Base::named(kind.to_tag().to_string())
        });
    }
    Some(jsx_name(argument).map_or_else(
        || Base {
            target: Some(Expression::new_identifier(SPAN, STYLED_BASE, ast_builder)),
            bound: Some(argument.clone_in(ast_builder.allocator())),
            ..Base::named(STYLED_BASE.to_string())
        },
        |name| Base {
            target: Some(argument.clone_in(ast_builder.allocator())),
            ..Base::named(name)
        },
    ))
}

/// How JSX names `expression` as a component: a binding that is not a tag name,
/// or a member of a binding such as `motion.div`
fn jsx_name(expression: &Expression<'_>) -> Option<String> {
    match expression {
        Expression::Identifier(ident)
            if !ident.name.starts_with(|c: char| c.is_ascii_lowercase()) =>
        {
            Some(ident.name.to_string())
        }
        Expression::StaticMemberExpression(member) => {
            let mut name = match unwrap_syntax_only(&member.object) {
                Expression::Identifier(ident) => ident.name.to_string(),
                object => jsx_name(object)?,
            };
            name.push('.');
            name.push_str(&member.property.name);
            Some(name)
        }
        _ => None,
    }
}

/// Resolve a `styled(...)` call to its base, and the index of the argument holding
/// the style object.
///
/// Two spellings build the same component: the curried `styled.div({...})` /
/// `styled("div")({...})`, whose callee already carries the tag, and the two-argument
/// `styled("div", {...})`, whose callee is the bare `styled` identifier.
fn resolve_styled_call_target<'a>(
    ast_builder: &AstBuilder<'a>,
    call: &CallExpression<'a>,
    imports: Kinds<'_>,
) -> Option<(Base<'a>, usize)> {
    if call.arguments.len() == 1
        && let Some(base) = extract_base_tag_and_class_name(ast_builder, &call.callee, imports)
    {
        return Some((base, 0));
    }
    // The style object must be a literal: it is the only way to tell `styled(tag, styles)`
    // apart from a malformed `styled("div", "span")`, which must be left untouched.
    if call.arguments.len() == 2
        && matches!(unwrap_syntax_only(&call.callee), Expression::Identifier(_))
        && call.arguments[1].as_expression().is_some_and(|styles| {
            matches!(unwrap_syntax_only(styles), Expression::ObjectExpression(_))
        })
        && let Some(base) = tag_from_argument(ast_builder, &call.arguments[0], imports)
    {
        return Some((base, 1));
    }
    None
}

/// Extract styles from styled function calls
/// Handles patterns like:
/// - styled.div`css`
/// - styled("div")`css`
/// - styled("div")({ bg: "red" })
/// - styled.div({ bg: "red" })
/// - styled(Component)({ bg: "red" })
pub fn extract_style_from_styled<'a>(
    ast_builder: &AstBuilder<'a>,
    expression: &mut Expression<'a>,
    naming: Naming<'_>,
    bindings: StyledBindings<'_>,
    modifiers: (&[Expression<'a>], Option<Forward>, &[u32]),
    inherited: Option<&StyledDefinition<'a>>,
    context: StyledContext<'_>,
) -> StyledExtraction<'a> {
    let imports = bindings.imports;
    let Naming {
        split_filename,
        markers,
    } = naming;
    let (attrs, forward, refused_array_controllers) = modifiers;
    let mut original_scope = expression.clone_in(ast_builder.allocator());
    let inherited = inherited.map(|definition| definition.rebind(ast_builder, &mut original_scope));
    let inherited = inherited.as_ref();
    let forward = combine_forward(inherited.and_then(|i| i.forward.as_ref()), forward);
    let mut reads = inherited.map_or_else(Reads::default, |inherited| inherited.reads.clone());
    for attr in attrs {
        reads.read_in(attr);
    }
    let mut composed_classes = Vec::new();
    let mut errors = Vec::new();
    errors.extend(crate::css_utils::object_layer_errors(expression, "styled"));
    let mut captured_bindings = Vec::new();
    let mut composed_rules = None;
    if let Expression::CallExpression(call) = expression
        && extract_base_tag_and_class_name(ast_builder, &call.callee, imports).is_some()
    {
        let prepared = arguments::prepare_with_bindings(
            ast_builder,
            call,
            (
                arguments::Context {
                    filename: context.filename,
                    scope: &mut original_scope,
                    reads: &mut reads,
                    refused: refused_array_controllers,
                },
                Some(&bindings),
            ),
        );
        composed_rules = prepared.rules;
        composed_classes = prepared.classes;
        captured_bindings = prepared.bindings;
        errors.extend(prepared.errors);
    }
    let (result, new_expr, definition) = if let Expression::TaggedTemplateExpression(tag) =
        expression
        && let Some(mut base) = extract_base_tag_and_class_name(ast_builder, &tag.tag, imports)
    {
        errors.extend(crate::css_utils::template_layer_errors(
            &tag.quasi, "styled",
        ));
        // Case 1: styled.div`css` or styled("div")`css`
        // Check if tag is styled.div or styled(...)
        // Extract CSS from template literal

        let mut invalid = Vec::new();
        unreadable_styles(
            &crate::dead_properties::template_errors(&tag.quasi),
            true,
            &mut invalid,
        );
        errors.extend(invalid.into_iter().map(|(offset, code, requirement)| {
            (
                offset,
                build_time_error("styled", &code, requirement.unwrap_or(STYLE_OBJECT)),
            )
        }));
        if !errors.is_empty() {
            return StyledExtraction {
                result: ExtractResult::default(),
                expression: Expression::new_string_literal(SPAN, "", None, ast_builder),
                errors,
                definition: None,
            };
        }

        for interpolation in &tag.quasi.expressions {
            reads.read_in(interpolation);
        }
        let template_composition::TemplateComposition {
            styles: own,
            statements,
            bindings: own_bindings,
            errors: template_errors,
        } = template_composition::compose_in(
            ast_builder,
            &tag.quasi,
            (&bindings, &original_scope, context.filename),
        );
        errors.extend(template_errors);
        let defaults = base.styles.take();
        let base = base.extending(ast_builder, inherited);
        let (mut props_styles, _) = compose_styles(ast_builder, inherited, defaults, own, None);
        let (mut classes, attrs) = inherited_parts(ast_builder, inherited, attrs);

        classes.extend(statements);
        let class_name = merge_expression_for_class_name(
            ast_builder,
            clone_all(ast_builder, &classes).chain(gen_class_names(
                ast_builder,
                &mut props_styles,
                None,
                split_filename,
            )),
        );
        let tag = Some(Expression::new_string_literal(
            SPAN,
            Str::from_in(&base.name, ast_builder.allocator()),
            None,
            ast_builder,
        ));
        let mut bindings = inherited.map_or_else(Vec::new, |definition| {
            definition
                .bindings
                .iter()
                .map(|(name, value)| (name.clone(), value.clone_in(ast_builder.allocator())))
                .collect()
        });
        bindings.extend(own_bindings);
        let withheld = withheld(
            &reads,
            inherited.map_or_else(
                || renders_tag(&base.name, base.bound.as_ref()),
                StyledDefinition::native,
            ),
            forward.as_ref(),
        );
        let definition = base
            .definition(
                ast_builder,
                classes,
                &props_styles,
                &attrs,
                (reads, forward),
                &bindings,
            )
            .with_execution(ast_builder, inherited)
            .with_css_slot(context.capability, inherited)
            .with_markers(markers, inherited);
        let staged = stages::create(
            ast_builder,
            &definition,
            stages::Context {
                base: &base,
                naming,
                jsx: context.jsx,
            },
        );
        let has_stage = staged.is_some();
        let component = staged.unwrap_or_else(|| {
            create_styled_component(
                ast_builder,
                &base,
                StyledRender {
                    class_name: &class_name,
                    style_vars: &gen_styles(ast_builder, &props_styles, None),
                    bindings,
                    withheld: &withheld,
                    capability: definition.capability.as_ref(),
                    execution: &definition.execution,
                    markers: definition.render_markers(ast_builder),
                },
                context.jsx,
            )
        });
        let styled_component = definition.retain_bound(
            ast_builder,
            base.render(
                ast_builder,
                if has_stage {
                    component
                } else {
                    apply_attrs(ast_builder, component, &attrs)
                },
            ),
        );

        let result = ExtractResult {
            styles: props_styles,
            tag,
            style_order: None,
            style_vars: None,
            props: None,
        };

        (Some(result), Some(styled_component), Some(definition))
    } else if let Expression::CallExpression(call) = expression
        && let Some((mut base, style_index)) =
            resolve_styled_call_target(ast_builder, call, imports)
    {
        // Case 2: styled.div({ bg: "red" }), styled("div")({ bg: "red" }),
        // or styled("div", { bg: "red" })

        reads.read_in(
            if let Argument::SpreadElement(spread) = &call.arguments[style_index] {
                &spread.argument
            } else {
                call.arguments[style_index].to_expression()
            },
        );
        let prepared = match &composed_rules {
            Some(_) => None,
            None => call.arguments[style_index]
                .as_expression()
                .and_then(|rules| prepare(ast_builder, rules, &original_scope)),
        };
        let mut bindings: Vec<_> = inherited.map_or_else(Vec::new, |definition| {
            definition
                .bindings
                .iter()
                .map(|(name, value)| (name.clone(), value.clone_in(ast_builder.allocator())))
                .collect()
        });
        bindings.extend(captured_bindings);
        bindings.extend(match prepared {
            Some(prepared) => {
                call.arguments[style_index] = Argument::from(prepared.rules);
                prepared.bindings
            }
            None => Vec::new(),
        });
        // Extract styles from object expression
        let mut authored = None;
        let ExtractResult {
            mut styles,
            style_order,
            style_vars,
            props,
            ..
        } = composed_rules.unwrap_or_else(|| {
            let (result, projection) = extract_with_order(
                ast_builder,
                None,
                if let Argument::SpreadElement(spread) = &mut call.arguments[style_index] {
                    &mut spread.argument
                } else {
                    call.arguments[style_index].to_expression_mut()
                },
                0,
                &None,
                LiteralHandling::ExpandResponsiveThemeToken,
            );
            authored = Some(projection.registered(context.filename));
            result
        });
        let mut unreadable = Vec::new();
        unreadable_styles(&styles, true, &mut unreadable);
        errors.extend(unreadable.into_iter().map(|(offset, code, requirement)| {
            (
                if offset == 0 { call.span.start } else { offset },
                build_time_error("styled", &code, requirement.unwrap_or(STYLE_OBJECT)),
            )
        }));
        if let Some(order) = style_order {
            for style in &mut styles {
                crate::composition::set_prop_order(style, order);
            }
            if let Some(authored) = &mut authored {
                for style in &mut authored.0 {
                    crate::composition::set_prop_order(style, order);
                }
            }
        }
        let defaults = base.styles.take();
        let base = base.extending(ast_builder, inherited);
        let (mut styles, composition_styles) =
            compose_styles(ast_builder, inherited, defaults, styles, authored);
        let (mut classes, attrs) = inherited_parts(ast_builder, inherited, attrs);
        classes.extend(composed_classes);

        let class_name = merge_expression_for_class_name(
            ast_builder,
            clone_all(ast_builder, &classes).chain(gen_class_names(
                ast_builder,
                &mut styles,
                None,
                split_filename,
            )),
        );
        let withheld = withheld(
            &reads,
            inherited.map_or_else(
                || renders_tag(&base.name, base.bound.as_ref()),
                StyledDefinition::native,
            ),
            forward.as_ref(),
        );
        let definition = base
            .definition(
                ast_builder,
                classes,
                &styles,
                &attrs,
                (reads, forward),
                &bindings,
            )
            .with_composition_styles(composition_styles)
            .with_execution(ast_builder, inherited)
            .with_css_slot(context.capability, inherited)
            .with_markers(markers, inherited);
        let staged = stages::create(
            ast_builder,
            &definition,
            stages::Context {
                base: &base,
                naming,
                jsx: context.jsx,
            },
        );
        let has_stage = staged.is_some();
        let component = staged.unwrap_or_else(|| {
            create_styled_component(
                ast_builder,
                &base,
                StyledRender {
                    class_name: &class_name,
                    style_vars: &gen_styles(ast_builder, &styles, None),
                    bindings,
                    withheld: &withheld,
                    capability: definition.capability.as_ref(),
                    execution: &definition.execution,
                    markers: definition.render_markers(ast_builder),
                },
                context.jsx,
            )
        });
        let styled_component = definition.retain_bound(
            ast_builder,
            base.render(
                ast_builder,
                if has_stage {
                    component
                } else {
                    apply_attrs(ast_builder, component, &attrs)
                },
            ),
        );

        let result = ExtractResult {
            styles,
            tag: None,
            style_order,
            style_vars,
            props,
        };

        (Some(result), Some(styled_component), Some(definition))
    } else {
        // Left as written it would call `styled` at runtime, which only the build runs
        let code = match &*expression {
            Expression::TaggedTemplateExpression(tag) => readable_code(&tag.tag),
            expression => readable_code(expression),
        };
        errors.push((
            expression.span().start,
            build_time_error("styled", &code, STYLED_FACTORY),
        ));
        (None, None, None)
    };
    StyledExtraction {
        result: result.unwrap_or_else(ExtractResult::default),
        expression: new_expr.unwrap_or_else(|| expression.clone_in(ast_builder.allocator())),
        errors,
        definition,
    }
}

/// What every module reads the same, besides the props a component takes
const SHARED_NAMES: &[&str] = &[
    "rest",
    "undefined",
    "NaN",
    "Infinity",
    "Math",
    "Number",
    "String",
    "Boolean",
    "Object",
    "Array",
    "JSON",
    "parseInt",
    "parseFloat",
    "isNaN",
    "isFinite",
];

/// The expressions a component evaluates for `prop`
fn style_expressions<'p, 'a>(prop: &'p ExtractStyleProp<'a>) -> Vec<&'p Expression<'a>> {
    match prop {
        ExtractStyleProp::Static(_) | ExtractStyleProp::Unreadable { .. } => Vec::new(),
        ExtractStyleProp::StaticArray(props) => props.iter().flat_map(style_expressions).collect(),
        ExtractStyleProp::Conditional {
            condition,
            consequent,
            alternate,
        } => std::iter::once(condition)
            .chain(consequent.iter().flat_map(|prop| style_expressions(prop)))
            .chain(alternate.iter().flat_map(|prop| style_expressions(prop)))
            .collect(),
        ExtractStyleProp::Enum { condition, map } => std::iter::once(condition)
            .chain(map.values().flatten().flat_map(style_expressions))
            .collect(),
        ExtractStyleProp::Expression { expression, .. } => vec![expression],
        ExtractStyleProp::MemberExpression { map, expression } => std::iter::once(expression)
            .chain(map.values().flat_map(|prop| style_expressions(prop)))
            .collect(),
    }
}

/// Whether the expressions `code` writes read nothing but the bindings they
/// declare, the props, and what every module reads the same
fn closed(code: impl Iterator<Item = String>) -> bool {
    let source = code
        .map(|code| ["(", crate::css_utils::rm_last_semi_colon(&code), ");\n"].concat())
        .collect::<String>();
    let allocator = oxc_allocator::Allocator::default();
    let ParserReturn {
        program,
        fatal_error,
        ..
    } = Parser::new(&allocator, &source, SourceType::tsx()).parse();
    let scoping = oxc_semantic::SemanticBuilder::new()
        .build(&program)
        .semantic
        .into_scoping();
    !fatal_error
        && scoping
            .root_unresolved_references()
            .iter()
            .all(|(name, references)| {
                SHARED_NAMES.contains(&name.as_str())
                    || references
                        .iter()
                        .all(|reference| !scoping.get_reference(*reference).is_value())
            })
}

fn clone_all<'s, 'a>(
    ast_builder: &'s AstBuilder<'a>,
    expressions: &'s [Expression<'a>],
) -> impl Iterator<Item = Expression<'a>> + 's {
    expressions
        .iter()
        .map(|expression| expression.clone_in_with_semantic_ids(ast_builder.allocator()))
}

/// The name the attrs wrapper binds props to, chosen not to shadow what the
/// attrs expressions read
const ATTRS_PROPS: &str = "__devupProps";

/// Strip styled-components' `.attrs()` / `.withConfig()` off a styled factory
/// such as `styled.div.attrs(a).withConfig(c)`, returning the attrs in the
/// order they apply, and the `withConfig` options in the order they apply
pub fn take_styled_modifiers<'a>(
    ast_builder: &AstBuilder<'a>,
    factory: &mut Expression<'a>,
    is_styled: impl Fn(&Expression<'_>) -> bool,
) -> (Vec<Expression<'a>>, Vec<Expression<'a>>) {
    let mut attrs = Vec::new();
    let mut configs = Vec::new();
    if !is_modified_styled(factory, is_styled) {
        return (attrs, configs);
    }
    while let Expression::CallExpression(call) = unwrap_syntax_only_mut(factory)
        && let CallExpression {
            callee, arguments, ..
        } = &mut **call
        && let Expression::StaticMemberExpression(member) = unwrap_syntax_only_mut(callee)
        && matches!(member.property.name.as_str(), "attrs" | "withConfig")
    {
        let placeholder = || Expression::new_null_literal(SPAN, ast_builder);
        if let Some(argument) = arguments[0].as_expression_mut() {
            let argument = std::mem::replace(argument, placeholder());
            if member.property.name == "attrs" {
                attrs.push(argument);
            } else {
                configs.push(argument);
            }
        }
        let object = std::mem::replace(&mut member.object, placeholder());
        *factory = object;
    }
    attrs.reverse();
    configs.reverse();
    (attrs, configs)
}

/// The `shouldForwardProp` the options objects give, the later applying after
/// the earlier, with the offset and code of one the build cannot evaluate
pub fn read_forward(options: &[&Expression<'_>]) -> (Option<Forward>, Option<(u32, String)>) {
    let mut forward = None;
    for options in options {
        let Expression::ObjectExpression(object) = unwrap_syntax_only(options) else {
            continue;
        };
        for property in &object.properties {
            let ObjectPropertyKind::ObjectProperty(property) = property else {
                continue;
            };
            if property.computed
                || property.key.static_name().as_deref() != Some("shouldForwardProp")
            {
                continue;
            }
            match Forward::read(&property.value) {
                Some(own) => forward = combine_forward(forward.as_ref(), Some(own)),
                None => {
                    return (
                        forward,
                        Some((
                            property.value.span().start,
                            build_time_error(
                                "styled",
                                &readable_code(&property.value),
                                SHOULD_FORWARD_PROP,
                            ),
                        )),
                    );
                }
            }
        }
    }
    (forward, None)
}

const SHOULD_FORWARD_PROP: &str = "`shouldForwardProp` must be a function of the prop name that compares it with strings, `[...].includes(prop)`, `prop.startsWith(...)` or `isPropValid(prop)`, joined by `!`, `&&` and `||`";

fn is_modified_styled(
    expression: &Expression<'_>,
    is_styled: impl Fn(&Expression<'_>) -> bool,
) -> bool {
    let mut expression = unwrap_syntax_only(expression);
    let mut modified = false;
    while let Some(object) = modifier_object(expression) {
        expression = unwrap_syntax_only(object);
        modified = true;
    }
    modified
        && match expression {
            Expression::StaticMemberExpression(member) => {
                matches!(&member.object, object @ Expression::Identifier(_) if is_styled(object))
            }
            Expression::CallExpression(call) => {
                matches!(&call.callee, callee @ Expression::Identifier(_) if is_styled(callee))
            }
            _ => false,
        }
}

fn modifier_object<'b, 'a>(expression: &'b Expression<'a>) -> Option<&'b Expression<'a>> {
    if let Expression::CallExpression(call) = expression
        && let [argument] = call.arguments.as_slice()
        && argument.is_expression()
        && let Expression::StaticMemberExpression(member) = unwrap_syntax_only(&call.callee)
        && matches!(member.property.name.as_str(), "attrs" | "withConfig")
    {
        Some(&member.object)
    } else {
        None
    }
}

/// Wrap `component` so the attrs are merged over its props first: an object is
/// spread, anything else is called with the props when it is a function, as
/// styled-components does
fn apply_attrs<'a>(
    ast_builder: &AstBuilder<'a>,
    component: Expression<'a>,
    attrs: &[Expression<'a>],
) -> Expression<'a> {
    if attrs.is_empty() {
        return component;
    }
    let merged = if attrs.iter().all(sets_plain_props) {
        spread_attrs(ast_builder, attrs)
    } else {
        merge_attrs(ast_builder, attrs)
    };
    props_arrow(
        ast_builder,
        wrap_direct_call(ast_builder, &component, &[merged]),
    )
}

/// The binding each attrs step reads the props merged so far through
const ATTRS_CONTEXT: &str = "__devupContext";
/// The binding each attrs step reads what its attrs give through
const ATTRS_VALUE: &str = "__devupAttrs";

/// Whether `attr` is an object setting props other than `className` and
/// `style` by name, which spreading over the props merges as attrs do
fn sets_plain_props(attr: &Expression<'_>) -> bool {
    let Expression::ObjectExpression(object) = unwrap_syntax_only(attr) else {
        return false;
    };
    object.properties.iter().all(|property| {
        matches!(property, ObjectPropertyKind::ObjectProperty(property)
            if !property.computed
                && property
                    .key
                    .static_name()
                    .is_some_and(|name| name != "className" && name != "style"))
    })
}

/// The props with each attrs spread over them in order
fn spread_attrs<'a>(ast_builder: &AstBuilder<'a>, attrs: &[Expression<'a>]) -> Expression<'a> {
    let mut merged = identifier(ast_builder, ATTRS_PROPS);
    for attr in attrs {
        merged = spread_objects(
            ast_builder,
            merged,
            attr.clone_in_with_semantic_ids(ast_builder.allocator()),
        );
    }
    merged
}

/// The props merged as styled-components merges attrs: each attrs, or what
/// calling it with the props merged so far gives, joins its `className` to
/// theirs, merges its `style` over theirs and replaces their other props; the
/// caller's `className` comes last
fn merge_attrs<'a>(ast_builder: &AstBuilder<'a>, attrs: &[Expression<'a>]) -> Expression<'a> {
    let mut merged = with_property(
        ast_builder,
        identifier(ast_builder, ATTRS_PROPS),
        "className",
        identifier(ast_builder, "undefined"),
    );
    for attr in attrs {
        let resolved = spread_objects(
            ast_builder,
            Expression::new_object_expression(
                SPAN,
                oxc_allocator::Vec::new_in(ast_builder),
                ast_builder,
            ),
            resolve_attrs(
                ast_builder,
                attr.clone_in_with_semantic_ids(ast_builder.allocator()),
            ),
        );
        let merge = call_with_values(
            ast_builder,
            vec![(ATTRS_VALUE.to_string(), resolved)],
            merge_attrs_value(ast_builder),
        );
        let step = named_arrow(ast_builder, ATTRS_CONTEXT, merge);
        merged = wrap_direct_call(ast_builder, &step, &[merged]);
    }
    let caller_class = joined_class(
        ast_builder,
        member(ast_builder, ATTRS_CONTEXT, "className"),
        member(ast_builder, ATTRS_PROPS, "className"),
    );
    let last = named_arrow(
        ast_builder,
        ATTRS_CONTEXT,
        with_property(
            ast_builder,
            identifier(ast_builder, ATTRS_CONTEXT),
            "className",
            caller_class,
        ),
    );
    wrap_direct_call(ast_builder, &last, &[merged])
}

/// What `attr` gives: itself when it is an object, or what calling it with
/// the props merged so far gives when it is a function
fn resolve_attrs<'a>(ast_builder: &AstBuilder<'a>, attr: Expression<'a>) -> Expression<'a> {
    if matches!(unwrap_syntax_only(&attr), Expression::ObjectExpression(_)) {
        return attr;
    }
    let called = wrap_direct_call(
        ast_builder,
        &attr,
        &[identifier(ast_builder, ATTRS_CONTEXT)],
    );
    if matches!(
        unwrap_syntax_only(&attr),
        Expression::ArrowFunctionExpression(_) | Expression::FunctionExpression(_)
    ) {
        return called;
    }
    let is_function = Expression::new_binary_expression(
        SPAN,
        Expression::new_unary_expression(
            SPAN,
            UnaryOperator::Typeof,
            attr.clone_in_with_semantic_ids(ast_builder.allocator()),
            ast_builder,
        ),
        BinaryOperator::StrictEquality,
        Expression::new_string_literal(SPAN, "function", None, ast_builder),
        ast_builder,
    );
    Expression::new_conditional_expression(SPAN, is_function, called, attr, ast_builder)
}

/// `{ ...context, ...attrs, className, style }` with the classes joined and
/// the styles merged
fn merge_attrs_value<'a>(ast_builder: &AstBuilder<'a>) -> Expression<'a> {
    let merged = spread_objects(
        ast_builder,
        identifier(ast_builder, ATTRS_CONTEXT),
        identifier(ast_builder, ATTRS_VALUE),
    );
    let class_name = joined_class(
        ast_builder,
        member(ast_builder, ATTRS_CONTEXT, "className"),
        member(ast_builder, ATTRS_VALUE, "className"),
    );
    let style = spread_objects(
        ast_builder,
        member(ast_builder, ATTRS_CONTEXT, "style"),
        member(ast_builder, ATTRS_VALUE, "style"),
    );
    let merged = with_property(ast_builder, merged, "className", class_name);
    with_property(ast_builder, merged, "style", style)
}

fn identifier<'a>(ast_builder: &AstBuilder<'a>, name: &'static str) -> Expression<'a> {
    Expression::new_identifier(SPAN, name, ast_builder)
}

fn member<'a>(
    ast_builder: &AstBuilder<'a>,
    object: &'static str,
    name: &'static str,
) -> Expression<'a> {
    Expression::StaticMemberExpression(oxc_ast::ast::StaticMemberExpression::boxed(
        SPAN,
        identifier(ast_builder, object),
        oxc_ast::ast::IdentifierName::new(SPAN, name, ast_builder),
        false,
        ast_builder,
    ))
}

/// `[first, second].filter(Boolean).join(" ") || undefined`
fn joined_class<'a>(
    ast_builder: &AstBuilder<'a>,
    first: Expression<'a>,
    second: Expression<'a>,
) -> Expression<'a> {
    let joined = wrap_array_filter(ast_builder, &[first, second])
        .unwrap_or_else(|| identifier(ast_builder, "undefined"));
    Expression::new_logical_expression(
        SPAN,
        joined,
        LogicalOperator::Or,
        identifier(ast_builder, "undefined"),
        ast_builder,
    )
}

/// `object` with `name` set to `value`: an object spreading it, then setting
/// the property
fn with_property<'a>(
    ast_builder: &AstBuilder<'a>,
    object: Expression<'a>,
    name: &'static str,
    value: Expression<'a>,
) -> Expression<'a> {
    let mut properties = match object {
        Expression::ObjectExpression(object) => object.unbox().properties,
        object => oxc_allocator::Vec::from_array_in(
            [ObjectPropertyKind::new_spread_property(
                SPAN,
                object,
                ast_builder,
            )],
            ast_builder,
        ),
    };
    properties.push(ObjectPropertyKind::new_object_property(
        SPAN,
        oxc_ast::ast::PropertyKind::Init,
        PropertyKey::new_static_identifier(SPAN, name, ast_builder),
        value,
        false,
        false,
        false,
        ast_builder,
    ));
    Expression::new_object_expression(SPAN, properties, ast_builder)
}

fn spread_objects<'a>(
    ast_builder: &AstBuilder<'a>,
    first: Expression<'a>,
    second: Expression<'a>,
) -> Expression<'a> {
    let mut properties = oxc_allocator::Vec::with_capacity_in(2, ast_builder);
    properties.push(ObjectPropertyKind::new_spread_property(
        SPAN,
        first,
        ast_builder,
    ));
    properties.push(ObjectPropertyKind::new_spread_property(
        SPAN,
        second,
        ast_builder,
    ));
    Expression::new_object_expression(SPAN, properties, ast_builder)
}

fn props_arrow<'a>(ast_builder: &AstBuilder<'a>, body: Expression<'a>) -> Expression<'a> {
    named_arrow(ast_builder, ATTRS_PROPS, body)
}

/// `(name) => body`
fn named_arrow<'a>(
    ast_builder: &AstBuilder<'a>,
    name: &'static str,
    body: Expression<'a>,
) -> Expression<'a> {
    let parameter = FormalParameter::new(
        SPAN,
        oxc_allocator::Vec::new_in(ast_builder),
        BindingPattern::new_binding_identifier(SPAN, name, ast_builder),
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
        None::<oxc_allocator::Box<Expression<'a>>>,
        false,
        None,
        false,
        false,
        ast_builder,
    );
    let params = FormalParameters::boxed(
        SPAN,
        FormalParameterKind::ArrowFormalParameters,
        oxc_allocator::Vec::from_iter_in([parameter], ast_builder),
        None::<oxc_allocator::Box<oxc_ast::ast::FormalParameterRest<'a>>>,
        ast_builder,
    );
    Expression::new_arrow_function_expression(
        SPAN,
        false,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterDeclaration<'a>>>,
        params,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
        body.into(),
        ast_builder,
    )
}

/// The binding the generated styled components forward refs through
pub const FORWARD_REF: &str = "__devupForwardRef";

/// `__devupForwardRef((p, ref) => component({ ...p, ref }))`: React 18 gives a
/// function component no `ref` prop, so the component takes it as one and
/// passes it to what it renders with the rest of its props
pub fn forward_ref<'a>(ast_builder: &AstBuilder<'a>, component: Expression<'a>) -> Expression<'a> {
    let parameter = |name: &'static str| {
        FormalParameter::new(
            SPAN,
            oxc_allocator::Vec::new_in(ast_builder),
            BindingPattern::new_binding_identifier(SPAN, name, ast_builder),
            None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
            None::<oxc_allocator::Box<Expression<'a>>>,
            false,
            None,
            false,
            false,
            ast_builder,
        )
    };
    let props = with_property(
        ast_builder,
        identifier(ast_builder, "__devupRefProps"),
        "ref",
        identifier(ast_builder, "__devupRef"),
    );
    let body = wrap_direct_call(
        ast_builder,
        &Expression::new_parenthesized_expression(SPAN, component, ast_builder),
        &[props],
    );
    let render = Expression::new_arrow_function_expression(
        SPAN,
        false,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterDeclaration<'a>>>,
        FormalParameters::boxed(
            SPAN,
            FormalParameterKind::ArrowFormalParameters,
            oxc_allocator::Vec::from_iter_in(
                [parameter("__devupRefProps"), parameter("__devupRef")],
                ast_builder,
            ),
            None::<oxc_allocator::Box<oxc_ast::ast::FormalParameterRest<'a>>>,
            ast_builder,
        ),
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
        body.into(),
        ast_builder,
    );
    wrap_direct_call(
        ast_builder,
        &identifier(ast_builder, FORWARD_REF),
        &[render],
    )
}

#[path = "styled_forwarding.rs"]
pub(crate) mod forwarding;

fn without_props<'a>(ast_builder: &AstBuilder<'a>, withheld: &[String]) -> Expression<'a> {
    forwarding::without_props(
        ast_builder,
        Expression::new_identifier(SPAN, "rest", ast_builder),
        withheld,
    )
}

/// The binding a styled component renders through: what `as` names, or the
/// tag or component it was defined with
const RENDERED: &str = "DevupAs";

struct StyledRender<'b, 'a> {
    class_name: &'b Option<Expression<'a>>,
    style_vars: &'b Option<Expression<'a>>,
    bindings: Vec<(String, Expression<'a>)>,
    withheld: &'b [String],
    capability: Option<&'b StyledCapability>,
    execution: &'b Execution<'a>,
    markers: Option<Expression<'a>>,
}

fn create_styled_component<'a>(
    ast_builder: &AstBuilder<'a>,
    base: &Base<'a>,
    render: StyledRender<'_, 'a>,
    helper: &mut Option<StyledJsx>,
) -> Expression<'a> {
    let StyledRender {
        class_name,
        style_vars,
        bindings,
        withheld,
        capability,
        execution,
        markers,
    } = render;
    let params = FormalParameters::boxed(
        SPAN,
        FormalParameterKind::ArrowFormalParameters,
        oxc_allocator::Vec::from_iter_in(
            vec![FormalParameter::new(
                SPAN,
                oxc_allocator::Vec::new_in(ast_builder),
                BindingPattern::new_object_pattern(
                    SPAN,
                    oxc_allocator::Vec::from_iter_in(
                        vec![
                            BindingProperty::new(
                                SPAN,
                                PropertyKey::new_static_identifier(SPAN, "style", ast_builder),
                                BindingPattern::new_binding_identifier(SPAN, "style", ast_builder),
                                true,
                                false,
                                ast_builder,
                            ),
                            BindingProperty::new(
                                SPAN,
                                PropertyKey::new_static_identifier(SPAN, "className", ast_builder),
                                BindingPattern::new_binding_identifier(
                                    SPAN,
                                    "className",
                                    ast_builder,
                                ),
                                true,
                                false,
                                ast_builder,
                            ),
                            BindingProperty::new(
                                SPAN,
                                PropertyKey::new_static_identifier(SPAN, "as", ast_builder),
                                BindingPattern::new_assignment_pattern(
                                    SPAN,
                                    BindingPattern::new_binding_identifier(
                                        SPAN,
                                        RENDERED,
                                        ast_builder,
                                    ),
                                    base.target.as_ref().map_or_else(
                                        || {
                                            Expression::new_string_literal(
                                                SPAN,
                                                Str::from_in(&base.name, ast_builder.allocator()),
                                                None,
                                                ast_builder,
                                            )
                                        },
                                        |target| target.clone_in(ast_builder.allocator()),
                                    ),
                                    ast_builder,
                                ),
                                false,
                                false,
                                ast_builder,
                            ),
                            BindingProperty::new(
                                SPAN,
                                PropertyKey::new_static_identifier(
                                    SPAN,
                                    "forwardedAs",
                                    ast_builder,
                                ),
                                BindingPattern::new_binding_identifier(
                                    SPAN,
                                    "forwardedAs",
                                    ast_builder,
                                ),
                                true,
                                false,
                                ast_builder,
                            ),
                        ],
                        ast_builder,
                    ),
                    Some(BindingRestElement::boxed(
                        SPAN,
                        BindingPattern::new_binding_identifier(SPAN, "rest", ast_builder),
                        ast_builder,
                    )),
                    ast_builder,
                ),
                None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
                None::<oxc_allocator::Box<Expression<'a>>>,
                false,
                None,
                false,
                false,
                ast_builder,
            )],
            ast_builder,
        ),
        None::<oxc_allocator::Box<oxc_ast::ast::FormalParameterRest<'a>>>,
        ast_builder,
    );
    let replacement = capability.map(|slot| slot.classes.classes(ast_builder, class_name.as_ref()));
    let delegated = match execution {
        Execution::Delegated(renderer) => Some(renderer),
        Execution::Replay
        | Execution::RetainedFixed { .. }
        | Execution::RetainedDynamic { .. }
        | Execution::Staged { .. } => None,
    };
    let own_classes = merge_expression_for_class_name(
        ast_builder,
        markers.into_iter().chain(
            replacement
                .as_ref()
                .or(class_name.as_ref())
                .filter(|_| delegated.is_none())
                .map(|classes| classes.clone_in(ast_builder.allocator())),
        ),
    );
    let class_name = own_classes.as_ref().map_or_else(
        || Expression::new_identifier(SPAN, "className", ast_builder),
        |name| {
            wrap_array_filter(
                ast_builder,
                &[
                    name.clone_in(ast_builder.allocator()),
                    Expression::new_identifier(SPAN, "className", ast_builder),
                ],
            )
            .unwrap_or_else(|| name.clone_in(ast_builder.allocator()))
        },
    );
    let style = style_vars.as_ref().map_or_else(
        || Expression::new_identifier(SPAN, "style", ast_builder),
        |style_vars| {
            merge_object_expressions(
                ast_builder,
                &[
                    style_vars.clone_in(ast_builder.allocator()),
                    Expression::new_identifier(SPAN, "style", ast_builder),
                ],
            )
            .unwrap_or_else(|| style_vars.clone_in(ast_builder.allocator()))
        },
    );
    let omissions = capability.map(|slot| {
        let mut omissions = withheld.to_vec();
        slot.omit(&mut omissions);
        omissions
    });
    let withheld = omissions.as_deref().unwrap_or(withheld);
    let ordinary =
        super::styled_transient::filter(ast_builder, without_props(ast_builder, withheld));
    let forwarded = capability.map_or_else(
        || ordinary.clone_in(ast_builder.allocator()),
        |slot| slot.forwarded(ast_builder, &ordinary),
    );
    let (target, rest, as_prop) = retained::transport(
        ast_builder,
        delegated,
        retained::Transport {
            capability,
            replacement: replacement.as_ref(),
            forwarded,
        },
    );
    let body = if let Some(helper) = helper {
        helper.render(ast_builder, target, (class_name, style, as_prop), rest)
    } else {
        Expression::new_jsx_element(
            SPAN,
            JSXOpeningElement::boxed(
                SPAN,
                JSXElementName::new_identifier(
                    SPAN,
                    Str::from_in(
                        delegated.map_or(RENDERED, |renderer| renderer.name.as_str()),
                        ast_builder.allocator(),
                    ),
                    ast_builder,
                ),
                None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterInstantiation<'a>>>,
                oxc_allocator::Vec::from_iter_in(
                    vec![
                        JSXAttributeItem::new_spread_attribute(SPAN, rest, ast_builder),
                        JSXAttributeItem::new_attribute(
                            SPAN,
                            JSXAttributeName::new_identifier(SPAN, "as", ast_builder),
                            Some(JSXAttributeValue::new_expression_container(
                                SPAN,
                                as_prop.into(),
                                ast_builder,
                            )),
                            ast_builder,
                        ),
                        JSXAttributeItem::new_attribute(
                            SPAN,
                            JSXAttributeName::new_identifier(SPAN, "className", ast_builder),
                            Some(JSXAttributeValue::new_expression_container(
                                SPAN,
                                class_name.into(),
                                ast_builder,
                            )),
                            ast_builder,
                        ),
                        JSXAttributeItem::new_attribute(
                            SPAN,
                            JSXAttributeName::new_identifier(SPAN, "style", ast_builder),
                            Some(JSXAttributeValue::new_expression_container(
                                SPAN,
                                style.into(),
                                ast_builder,
                            )),
                            ast_builder,
                        ),
                    ],
                    ast_builder,
                ),
                ast_builder,
            ),
            oxc_allocator::Vec::new_in(ast_builder),
            None::<oxc_allocator::Box<oxc_ast::ast::JSXClosingElement<'a>>>,
            ast_builder,
        )
    };
    Expression::new_arrow_function_expression(
        SPAN,
        false,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeParameterDeclaration<'a>>>,
        params,
        None::<oxc_allocator::Box<oxc_ast::ast::TSTypeAnnotation<'a>>>,
        bind_render_body(ast_builder, bindings, body),
        ast_builder,
    )
}

#[cfg(test)]
#[path = "w27_props_rule_choices_tests.rs"]
mod w27_props_rule_choices_tests;

#[cfg(test)]
#[path = "w27_styled_linux_coverage.rs"]
mod w27_styled_linux_coverage;

#[cfg(test)]
#[path = "w27_styled_known_mixins_tests.rs"]
mod w27_styled_known_mixins_tests;

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use std::collections::BTreeMap;

    use oxc_allocator::Allocator;

    use super::*;

    #[test]
    fn style_expressions_find_what_every_kind_of_style_evaluates() {
        let allocator = Allocator::default();
        let ast_builder = AstBuilder::new(&allocator);
        let name = |name: &str| {
            Expression::new_identifier(SPAN, Str::from_in(name, &allocator), &ast_builder)
        };
        let evaluated = |expression| ExtractStyleProp::Expression {
            styles: vec![],
            expression,
        };
        let props = [
            ExtractStyleProp::Static(ExtractStyleValue::Typography("t".to_string())),
            ExtractStyleProp::Unreadable {
                offset: 0,
                code: String::new(),
                prop: false,
                requirement: None,
            },
            ExtractStyleProp::StaticArray(vec![evaluated(name("a"))]),
            ExtractStyleProp::Conditional {
                condition: name("b"),
                consequent: Some(Box::new(evaluated(name("c")))),
                alternate: Some(Box::new(evaluated(name("d")))),
            },
            ExtractStyleProp::Enum {
                condition: name("e"),
                map: BTreeMap::from([("x".to_string(), vec![evaluated(name("f"))])]),
            },
            ExtractStyleProp::MemberExpression {
                map: BTreeMap::from([("k".to_string(), Box::new(evaluated(name("h"))))]),
                expression: name("g"),
            },
        ];
        let code: Vec<String> = props
            .iter()
            .flat_map(style_expressions)
            .map(expression_to_code)
            .collect();
        assert_eq!(code, ["a;", "b;", "c;", "d;", "e;", "f;", "g;", "h;"]);
    }

    #[test]
    fn code_is_closed_when_it_reads_only_what_it_declares() {
        let closed_code = |code: &[&str]| closed(code.iter().map(ToString::to_string));
        assert!(closed_code(&[]));
        assert!(closed_code(&[
            "(p) => p.x * Math.PI",
            "((p) => p.on)(rest)"
        ]));
        assert!(closed_code(&["(p: Props) => p.x"]));
        assert!(closed_code(&["({ id: `known` });", "((p) => p.on)(rest);"]));
        assert!(!closed_code(&["({ onClick: handler });"]));
        assert!(!closed_code(&["(p) => scale(p.x)"]));
        assert!(!closed_code(&["(p) => p.x", "window.mode"]));
        assert!(!closed_code(&["@#$"]));
    }
}
