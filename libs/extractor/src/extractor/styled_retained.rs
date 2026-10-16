use super::{
    RENDERED, STYLED_BASE, StyledCapability, StyledDefinition, call_with_values, fixed, identifier,
    jsx_name, marker_class, merge_expression_for_class_name, renders_tag, with_property,
    without_props,
};
use oxc_allocator::{Allocator, CloneIn, GetAllocator};
use oxc_ast::{
    ast::{Expression, ObjectPropertyKind},
    builder::AstBuilder,
};
use oxc_span::SPAN;

pub(super) enum Execution<'a> {
    Replay,
    RetainedFixed {
        native: bool,
        capability: StyledCapability,
    },
    Delegated(Renderer<'a>),
}

pub(super) struct Renderer<'a> {
    pub name: String,
    pub target: Expression<'a>,
    pub bound: Option<Expression<'a>>,
    pub capability: StyledCapability,
    pub native: bool,
    pub identities: Vec<String>,
}

impl Execution<'_> {
    pub(super) const fn replays(&self) -> bool {
        match self {
            Self::Replay => true,
            Self::RetainedFixed { .. } | Self::Delegated(_) => false,
        }
    }

    pub(super) fn clone_in<'b>(&self, allocator: &'b Allocator) -> Execution<'b> {
        match self {
            Self::Replay => Execution::Replay,
            Self::RetainedFixed { native, capability } => Execution::RetainedFixed {
                native: *native,
                capability: capability.clone(),
            },
            Self::Delegated(renderer) => Execution::Delegated(Renderer {
                name: renderer.name.clone(),
                target: renderer.target.clone_in(allocator),
                bound: renderer
                    .bound
                    .as_ref()
                    .map(|value| value.clone_in(allocator)),
                capability: renderer.capability.clone(),
                native: renderer.native,
                identities: renderer.identities.clone(),
            }),
        }
    }
}

impl<'a> StyledDefinition<'a> {
    pub(crate) fn shared(mut self) -> Option<Self> {
        if self.portable() {
            return Some(self);
        }
        if self.fixed_summary()
            && let Some(capability) = self.capability.take()
        {
            let native = self.native();
            self.name = String::new();
            self.target = None;
            self.bound = None;
            self.bindings.clear();
            self.execution = Execution::RetainedFixed {
                native,
                capability: capability.clone(),
            };
            self.capability = Some(capability);
            return Some(self);
        }
        None
    }

    pub(crate) fn used_as(&self, ast: &AstBuilder<'a>, target: &Expression<'a>) -> Self {
        let mut definition = self.clone_in(ast.allocator());
        match &self.execution {
            Execution::RetainedFixed { native, capability } => {
                let name = jsx_name(target);
                definition.execution = Execution::Delegated(Renderer {
                    name: name.clone().unwrap_or_else(|| STYLED_BASE.to_string()),
                    target: name.as_ref().map_or_else(
                        || identifier(ast, STYLED_BASE),
                        |_| target.clone_in(ast.allocator()),
                    ),
                    bound: name.is_none().then(|| target.clone_in(ast.allocator())),
                    capability: capability.clone(),
                    native: *native,
                    identities: self
                        .markers
                        .iter()
                        .chain(&self.inherited_markers)
                        .cloned()
                        .collect(),
                });
                definition.target = Some(identifier(ast, "undefined"));
            }
            Execution::Replay | Execution::Delegated(_) => {}
        }
        definition
    }

    pub(super) fn fixed_summary(&self) -> bool {
        self.attrs.is_empty() && self.classes.is_empty() && self.styles.iter().all(fixed)
    }

    pub(super) fn native(&self) -> bool {
        match &self.execution {
            Execution::Replay => renders_tag(&self.name, self.bound.as_ref()),
            Execution::RetainedFixed { native, .. } => *native,
            Execution::Delegated(renderer) => renderer.native,
        }
    }

    pub(super) fn with_execution(mut self, ast: &AstBuilder<'a>, inherited: Option<&Self>) -> Self {
        if let Some(inherited) = inherited {
            self.execution = inherited.execution.clone_in(ast.allocator());
        }
        self
    }

    pub(super) fn render_markers(&self, ast: &AstBuilder<'a>) -> Option<Expression<'a>> {
        let inherited = match &self.execution {
            Execution::Delegated(renderer) => renderer.identities.as_slice(),
            Execution::Replay | Execution::RetainedFixed { .. } => &[],
        };
        merge_expression_for_class_name(
            ast,
            self.markers
                .iter()
                .chain(&self.inherited_markers)
                .filter(|marker| !inherited.contains(marker))
                .map(|marker| marker_class(ast, marker)),
        )
    }

    pub(super) fn retain_bound(
        &self,
        ast: &AstBuilder<'a>,
        component: Expression<'a>,
    ) -> Expression<'a> {
        match &self.execution {
            Execution::Delegated(renderer) => match &renderer.bound {
                Some(bound) => call_with_values(
                    ast,
                    vec![(STYLED_BASE.to_string(), bound.clone_in(ast.allocator()))],
                    component,
                ),
                None => component,
            },
            Execution::Replay | Execution::RetainedFixed { .. } => component,
        }
    }
}

pub(super) struct Transport<'b, 'a> {
    pub capability: Option<&'b StyledCapability>,
    pub replacement: Option<&'b Expression<'a>>,
    pub forwarded: Expression<'a>,
}

pub(super) fn transport<'a>(
    ast: &AstBuilder<'a>,
    renderer: Option<&Renderer<'a>>,
    transport: Transport<'_, 'a>,
) -> (Expression<'a>, Expression<'a>, Expression<'a>) {
    match renderer {
        None => (
            identifier(ast, RENDERED),
            transport.forwarded,
            identifier(ast, "forwardedAs"),
        ),
        Some(renderer) => {
            let mut controls = Vec::new();
            if let Some(capability) = transport.capability {
                capability.omit(&mut controls);
            }
            let raw = with_property(
                ast,
                without_props(ast, &controls),
                "forwardedAs",
                identifier(ast, "forwardedAs"),
            );
            let raw = Expression::new_object_expression(
                SPAN,
                oxc_allocator::Vec::from_array_in(
                    [
                        ObjectPropertyKind::new_spread_property(SPAN, raw, ast),
                        renderer.capability.classes.property(
                            ast,
                            transport
                                .replacement
                                .map(|value| value.clone_in(ast.allocator())),
                        ),
                        renderer
                            .capability
                            .forwarded
                            .property(ast, Some(transport.forwarded)),
                    ],
                    ast,
                ),
                ast,
            );
            (
                renderer.target.clone_in(ast.allocator()),
                raw,
                identifier(ast, RENDERED),
            )
        }
    }
}
