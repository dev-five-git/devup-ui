use css::{
    at_rule::{MediaCombination, combine_media_queries, normalize_query},
    style_selector::{AtRule, AtRuleKind, StyleSelector},
};

#[path = "own_css_key.rs"]
mod key;
#[path = "own_css_selector.rs"]
mod selector;
#[cfg(test)]
#[path = "own_css_selector_tests.rs"]
mod tests;
pub(crate) use key::under;

pub(crate) enum Frame {
    Selector(String),
    At(AtRuleKind, String),
    Layer(String),
    Params(String),
    UncertainParams,
}

struct Link {
    parent: Option<usize>,
    frame: Frame,
    selected: bool,
    target: std::cell::OnceCell<Option<usize>>,
}

#[derive(Default)]
pub(crate) struct Contexts {
    links: Vec<Link>,
    targets: selector::Targets,
}

impl Contexts {
    pub fn push(&mut self, parent: Option<usize>, frame: Frame) -> usize {
        let selected = self.selected(parent) || matches!(frame, Frame::Selector(_) | Frame::At(..));
        let id = self.links.len();
        self.links.push(Link {
            parent,
            frame,
            selected,
            target: std::cell::OnceCell::new(),
        });
        id
    }

    pub fn selected(&self, path: Option<usize>) -> bool {
        path.is_some_and(|id| self.links[id].selected)
    }

    pub fn uncertain(&mut self, path: Option<usize>) -> usize {
        self.push(path, Frame::UncertainParams)
    }

    pub fn requires_argument(&mut self, path: Option<usize>) -> bool {
        let mut pending = Vec::new();
        let mut cursor = path;
        let mut target = None;
        while let Some(id) = cursor {
            if let Some(cached) = self.links[id].target.get().copied() {
                target = cached;
                break;
            }
            pending.push(id);
            cursor = self.links[id].parent;
        }
        for id in pending.into_iter().rev() {
            target = match &self.links[id].frame {
                Frame::At(..) | Frame::Layer(_) => target,
                Frame::Selector(template) => Some(self.targets.nest(target, template)),
                Frame::Params(_) => self.targets.consume(target),
                Frame::UncertainParams => Some(self.targets.uncertain()),
            };
            self.links[id].target = std::cell::OnceCell::from(target);
        }
        self.targets.required(target)
    }

    pub fn materialize(
        &self,
        path: Option<usize>,
    ) -> Option<(Option<StyleSelector>, Option<String>)> {
        let mut links = Vec::new();
        let mut cursor = path;
        while let Some(id) = cursor {
            links.push(&self.links[id].frame);
            cursor = self.links[id].parent;
        }
        let mut selector = selector::SelectorText::default();
        let mut wrappers: Vec<AtRule> = Vec::new();
        let mut layers = Vec::new();
        for frame in links.into_iter().rev() {
            match frame {
                Frame::Selector(template) => selector.nest(template),
                Frame::At(kind, query) => {
                    let query = normalize_query(query);
                    if let Some(last) = wrappers.last_mut()
                        && last.kind == AtRuleKind::Media
                        && *kind == AtRuleKind::Media
                    {
                        match combine_media_queries(&last.query, &query) {
                            MediaCombination::Merged(query) => {
                                last.query = query;
                                continue;
                            }
                            MediaCombination::Never => return None,
                            MediaCombination::Nest => {}
                        }
                    }
                    wrappers.push(AtRule { kind: *kind, query });
                }
                Frame::Layer(layer) => layers.push(layer.as_str()),
                Frame::Params(params) => selector.params(params),
                Frame::UncertainParams => {}
            }
        }
        let selector = selector.text();
        let selector = if let Some(last) = wrappers.pop() {
            Some(StyleSelector::At {
                kind: last.kind,
                query: last.query,
                selector,
                outer: wrappers,
                file: None,
            })
        } else {
            selector.map(StyleSelector::Selector)
        };
        Some((selector, (!layers.is_empty()).then(|| layers.join("."))))
    }
}
