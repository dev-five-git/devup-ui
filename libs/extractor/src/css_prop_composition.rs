use super::{CssValue, DevupVisitor, css_prop_override_error, overlaps};
use crate::{ExtractStyleProp, extractor::styled_css_slot::CssPropLowering};
use oxc_allocator::GetAllocator;
use oxc_ast::ast::Expression;

enum OwnStyles<'a> {
    Replay(Vec<ExtractStyleProp<'a>>),
    Residual(crate::composition::Coverage),
}

impl<'a> DevupVisitor<'a> {
    /// The styles the `css` prop of `element` gives it. When the element is a
    /// styled component the file binds to `symbol`, fixed own styles compose
    /// under it if `renders`: capture-free definitions inline their tag,
    /// captured definitions keep their renderer through its private slot.
    /// `None` when the build cannot order them.
    pub(super) fn css_prop_styles(
        &mut self,
        names: (&str, &str),
        css: CssValue<'a>,
        symbol: Option<oxc_syntax::symbol::SymbolId>,
        renders: bool,
        class_name: Option<&Expression<'a>>,
    ) -> Option<(CssPropLowering, Vec<ExtractStyleProp<'a>>)> {
        let (element, lookup) = names;
        let allocator = self.ast.allocator();
        let clone = |styles: &[ExtractStyleProp<'a>]| -> Vec<ExtractStyleProp<'a>> {
            styles
                .iter()
                .map(|style| style.clone_in(allocator))
                .collect()
        };
        let definition = self.css_definition(symbol, lookup);
        let markers = definition.and_then(|definition| definition.marker_classes(&self.ast));
        let composed = definition.filter(|_| renders).and_then(|definition| {
            definition
                .inline()
                .map(|(tag, _)| {
                    (
                        CssPropLowering::Inline(tag.to_string()),
                        clone(definition.composition_styles()),
                    )
                })
                .or_else(|| {
                    definition.css_slot().map(|slot| {
                        (
                            CssPropLowering::Slot(slot.clone()),
                            clone(definition.composition_styles()),
                        )
                    })
                })
        });
        if let Some((lowering, styles)) = composed {
            let markers = match &lowering {
                CssPropLowering::Inline(_) => markers,
                CssPropLowering::Retained | CssPropLowering::Slot(_) => None,
            };
            let mut props = self.compose_css_prop(element, css, styles, class_name);
            props.extend(markers.map(|expression| ExtractStyleProp::Expression {
                expression,
                styles: vec![],
            }));
            return Some((lowering, props));
        }
        let own = definition.map(|definition| {
            definition.css_coverage().map_or_else(
                || OwnStyles::Replay(clone(definition.composition_styles())),
                OwnStyles::Residual,
            )
        });
        let offset = css.offset;
        let props = self.compose_css_prop(element, css, vec![], class_name);
        if own.is_some_and(|own| match own {
            OwnStyles::Replay(styles) => overlaps(&styles, &props),
            OwnStyles::Residual(coverage) => coverage.overlaps(&props),
        }) {
            self.errors.push((offset, css_prop_override_error(element)));
            return None;
        }
        Some((CssPropLowering::Retained, props))
    }
}
