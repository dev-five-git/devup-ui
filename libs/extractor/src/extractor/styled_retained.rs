use super::{
    STYLED_BASE, StyledCapability, StyledDefinition, call_with_values, fixed, identifier, jsx_name,
    marker_class, merge_expression_for_class_name, renders_tag,
};
use oxc_allocator::{Allocator, CloneIn, GetAllocator};
use oxc_ast::{ast::Expression, builder::AstBuilder};
#[path = "styled_retained_transport.rs"]
mod scalar_transport;
pub(super) use scalar_transport::{Transport, transport};

pub(super) enum Execution<'a> {
    Replay,
    RetainedFixed {
        native: bool,
        capability: StyledCapability,
    },
    RetainedDynamic {
        native: bool,
        capability: StyledCapability,
        shape: super::residual::Shape,
    },
    Delegated(Renderer<'a>),
    Staged {
        renderer: Renderer<'a>,
        shape: super::residual::Shape,
        retarget: Option<Expression<'a>>,
    },
}

pub(super) struct Renderer<'a> {
    pub name: String,
    pub target: Expression<'a>,
    pub bound: Option<Expression<'a>>,
    pub capability: StyledCapability,
    pub native: bool,
    pub identities: Vec<String>,
}

impl Renderer<'_> {
    fn clone_in<'b>(&self, allocator: &'b Allocator) -> Renderer<'b> {
        Renderer {
            name: self.name.clone(),
            target: self.target.clone_in(allocator),
            bound: self.bound.as_ref().map(|value| value.clone_in(allocator)),
            capability: self.capability.clone(),
            native: self.native,
            identities: self.identities.clone(),
        }
    }
}

impl Execution<'_> {
    pub(super) const fn replays(&self) -> bool {
        match self {
            Self::Replay => true,
            Self::RetainedFixed { .. }
            | Self::RetainedDynamic { .. }
            | Self::Delegated(_)
            | Self::Staged { .. } => false,
        }
    }

    pub(super) fn clone_in<'b>(&self, allocator: &'b Allocator) -> Execution<'b> {
        match self {
            Self::Replay => Execution::Replay,
            Self::RetainedFixed { native, capability } => Execution::RetainedFixed {
                native: *native,
                capability: capability.clone(),
            },
            Self::RetainedDynamic {
                native,
                capability,
                shape,
            } => Execution::RetainedDynamic {
                native: *native,
                capability: capability.clone(),
                shape: shape.clone(),
            },
            Self::Staged {
                renderer,
                shape,
                retarget,
            } => Execution::Staged {
                renderer: renderer.clone_in(allocator),
                shape: shape.clone(),
                retarget: retarget.as_ref().map(|value| value.clone_in(allocator)),
            },
            Self::Delegated(renderer) => Execution::Delegated(renderer.clone_in(allocator)),
        }
    }
}

impl<'a> StyledDefinition<'a> {
    pub(crate) fn shared(mut self) -> Option<Self> {
        if self.portable() {
            return Some(self);
        }
        if !self.fixed_summary()
            && let Some(capability) = self.capability.clone()
        {
            let shape = self.residual_shape();
            let native = self.native();
            self.name = String::new();
            self.target = None;
            self.bound = None;
            self.bindings.clear();
            self.attrs.clear();
            self.classes.clear();
            self.styles.clear();
            self.composition_styles = None;
            self.execution = Execution::RetainedDynamic {
                native,
                capability,
                shape,
            };
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
            Execution::RetainedDynamic {
                native,
                capability,
                shape,
            } => {
                definition.execution = Execution::Staged {
                    renderer: Renderer {
                        name: jsx_name(target).unwrap_or_else(|| STYLED_BASE.to_string()),
                        target: target.clone_in(ast.allocator()),
                        bound: None,
                        capability: capability.clone(),
                        native: *native,
                        identities: self
                            .markers
                            .iter()
                            .chain(&self.inherited_markers)
                            .cloned()
                            .collect(),
                    },
                    shape: shape.clone(),
                    retarget: None,
                };
                definition.target = Some(identifier(ast, "undefined"));
            }
            Execution::Replay | Execution::Delegated(_) | Execution::Staged { .. } => {}
        }
        definition
    }

    pub(super) fn fixed_summary(&self) -> bool {
        !matches!(
            self.execution,
            Execution::RetainedDynamic { .. } | Execution::Staged { .. }
        ) && self.attrs.is_empty()
            && self.classes.is_empty()
            && self.styles.iter().all(fixed)
    }

    pub(super) fn native(&self) -> bool {
        match &self.execution {
            Execution::Replay => renders_tag(&self.name, self.bound.as_ref()),
            Execution::RetainedFixed { native, .. } | Execution::RetainedDynamic { native, .. } => {
                *native
            }
            Execution::Delegated(renderer) | Execution::Staged { renderer, .. } => renderer.native,
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
            Execution::Delegated(renderer) | Execution::Staged { renderer, .. } => {
                renderer.identities.as_slice()
            }
            Execution::Replay
            | Execution::RetainedFixed { .. }
            | Execution::RetainedDynamic { .. } => &[],
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
            Execution::Delegated(renderer) | Execution::Staged { renderer, .. } => {
                match &renderer.bound {
                    Some(bound) => call_with_values(
                        ast,
                        vec![(STYLED_BASE.to_string(), bound.clone_in(ast.allocator()))],
                        component,
                    ),
                    None => component,
                }
            }
            Execution::Replay
            | Execution::RetainedFixed { .. }
            | Execution::RetainedDynamic { .. } => component,
        }
    }
}
