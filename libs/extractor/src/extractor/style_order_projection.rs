//! Authored chunks retained during extraction, independently of legacy emission order.
use super::{
    ExtractResult,
    extract_style_from_expression::{self, LiteralHandling},
};
use crate::{ExtractStyleProp, composition::Composition};
use css::style_selector::StyleSelector;
use oxc_allocator::{CloneIn, GetAllocator};
use oxc_ast::{ast::Expression, builder::AstBuilder};

pub(crate) struct AuthoredStyles<'a>(pub(crate) Vec<ExtractStyleProp<'a>>);

impl<'a> AuthoredStyles<'a> {
    /// Capture authored first use before a fold can erase its first declaration.
    pub(crate) fn registered(mut self, file: &str) -> Self {
        let mut collector = crate::style_collector::StyleCollector::default();
        collector.use_layers(&self.0);
        let registrations = collector.into_styles(file);
        self.0.splice(
            0..0,
            registrations.into_iter().map(ExtractStyleProp::Static),
        );
        self
    }

    pub(crate) fn clone_in<'b>(
        &self,
        allocator: &'b oxc_allocator::Allocator,
    ) -> AuthoredStyles<'b> {
        AuthoredStyles(self.0.iter().map(|prop| prop.clone_in(allocator)).collect())
    }

    pub(crate) fn normalized(self, ast: &AstBuilder<'a>) -> Self {
        let mut composition = Composition::default();
        self.apply(ast, &mut composition);
        let mut styles = composition.into_props();
        extract_style_from_expression::yield_typography(&mut styles);
        Self(styles)
    }

    pub(crate) fn apply(self, ast: &AstBuilder<'a>, composition: &mut Composition<'a>) {
        for prop in self.0 {
            match prop {
                ExtractStyleProp::StaticArray(props) => Self(props).apply(ast, composition),
                ExtractStyleProp::Conditional {
                    condition,
                    consequent,
                    alternate,
                } => {
                    let normalize = |side: Option<Box<ExtractStyleProp<'a>>>| {
                        Self(side.into_iter().map(|prop| *prop).collect())
                            .normalized(ast)
                            .0
                    };
                    composition.apply_conditional(
                        ast,
                        &condition,
                        normalize(consequent),
                        normalize(alternate),
                    );
                }
                prop => composition.apply(ast, vec![prop]),
            }
        }
    }
}

#[derive(Default)]
pub(super) struct Projection<'a> {
    children: Option<Vec<AuthoredStyles<'a>>>,
    root_chunks: Option<Vec<AuthoredStyles<'a>>>,
    branches: bool,
    swapped: bool,
}

impl<'a> Projection<'a> {
    pub(super) fn enabled() -> Self {
        Self {
            children: Some(Vec::new()),
            ..Self::default()
        }
    }

    pub(super) fn extract(
        &mut self,
        ast: &AstBuilder<'a>,
        name: Option<&str>,
        expression: &mut Expression<'a>,
        level: u8,
        selector: &Option<StyleSelector>,
        handling: LiteralHandling,
    ) -> ExtractResult<'a> {
        let mut child = if self.children.is_some() {
            Self::enabled()
        } else {
            Self::default()
        };
        let result = extract_style_from_expression::extract_projected(
            ast, name, expression, level, selector, handling, &mut child,
        );
        if let Some(children) = &mut self.children {
            children.push(child.finish(ast, &result.styles));
        }
        result
    }

    pub(super) fn root(&mut self) {
        if self.children.is_some() {
            self.root_chunks = Some(Vec::new());
        }
    }

    pub(super) fn end_property(&mut self) {
        if let (Some(children), Some(chunks)) = (&mut self.children, &mut self.root_chunks) {
            chunks.push(AuthoredStyles(
                children.drain(..).flat_map(|chunk| chunk.0).collect(),
            ));
        }
    }

    pub(super) const fn conditional(&mut self, swapped: bool) {
        self.branches = true;
        self.swapped = swapped;
    }

    pub(super) fn record(&mut self, ast: &AstBuilder<'a>, prop: &ExtractStyleProp<'a>) {
        if let Some(children) = &mut self.children {
            children.push(AuthoredStyles(vec![prop.clone_in(ast.allocator())]));
        }
    }

    pub(super) fn layer(&mut self, layer: &str) {
        if let Some(child) = self
            .children
            .as_mut()
            .and_then(|children| children.last_mut())
        {
            extract_style_from_expression::place_in_layer(&mut child.0, layer);
        }
    }

    pub(super) fn finish(
        self,
        ast: &AstBuilder<'a>,
        raw: &[ExtractStyleProp<'a>],
    ) -> AuthoredStyles<'a> {
        if let Some(chunks) = self.root_chunks {
            return AuthoredStyles(chunks.into_iter().rev().flat_map(|chunk| chunk.0).collect());
        }
        let children = self.children.unwrap_or_default();
        if children.is_empty() {
            return AuthoredStyles(
                raw.iter()
                    .map(|prop| prop.clone_in(ast.allocator()))
                    .collect(),
            );
        }
        if self.branches
            && let [
                ExtractStyleProp::Conditional {
                    condition,
                    consequent,
                    alternate,
                },
            ] = raw
        {
            let mut children = children.into_iter();
            let mut side = || {
                children
                    .next()
                    .map(|child| Box::new(ExtractStyleProp::StaticArray(child.0)))
            };
            let (yes, no) = if self.swapped {
                let no = side();
                (side(), no)
            } else {
                let yes = consequent.as_ref().and_then(|_| side());
                (yes, alternate.as_ref().and_then(|_| side()))
            };
            return AuthoredStyles(vec![ExtractStyleProp::Conditional {
                condition: condition.clone_in(ast.allocator()),
                consequent: yes,
                alternate: no,
            }]);
        }
        AuthoredStyles(children.into_iter().flat_map(|child| child.0).collect())
    }
}

pub(crate) fn extract_with_order<'a>(
    ast: &AstBuilder<'a>,
    name: Option<&str>,
    expression: &mut Expression<'a>,
    level: u8,
    selector: &Option<StyleSelector>,
    handling: LiteralHandling,
) -> (ExtractResult<'a>, AuthoredStyles<'a>) {
    let mut projection = Projection::enabled();
    let result = extract_style_from_expression::extract_projected(
        ast,
        name,
        expression,
        level,
        selector,
        handling,
        &mut projection,
    );
    let authored = projection.finish(ast, &result.styles);
    (result, authored)
}

#[cfg(test)]
#[path = "style_order_projection_tests.rs"]
mod tests;
