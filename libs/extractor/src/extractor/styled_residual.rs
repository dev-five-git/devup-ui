use super::{
    AstBuilder, Execution, Expression, ExtractStyleProp, GetAllocator, Naming, StyledDefinition,
    gen_class_names, merge_expression_for_class_name,
};
use crate::composition::{CascadeKey, Composition, Coverage, coverage};

impl StyledDefinition<'_> {
    pub(crate) fn css_coverage(&self) -> Option<Coverage> {
        match &self.execution {
            Execution::RetainedDynamic { shape, .. } => Some(shape.1.clone()),
            Execution::Staged { shape, .. } => {
                Some(shape.1.union(&coverage(self.composition_styles())))
            }
            Execution::Replay | Execution::RetainedFixed { .. } | Execution::Delegated(_) => None,
        }
    }

    pub(super) fn residual_shape(&self) -> Shape {
        let arena = oxc_allocator::Allocator::default();
        let ast = AstBuilder::new(&arena);
        let own = shape(&ast, self.composition_styles());
        match &self.execution {
            Execution::Staged { shape, .. } => shape.union(&own),
            Execution::RetainedDynamic { shape, .. } => shape.clone(),
            Execution::Replay | Execution::RetainedFixed { .. } | Execution::Delegated(_) => own,
        }
    }
}

#[derive(Clone)]
pub(super) struct Shape(pub Vec<Option<CascadeKey>>, Coverage);

#[cfg(test)]
#[path = "styled_residual_tests.rs"]
mod tests;

pub(super) struct Selections<'a> {
    pub shape: Shape,
    pub values: Vec<Option<Expression<'a>>>,
}

pub(super) fn shape(ast: &AstBuilder<'_>, styles: &[ExtractStyleProp<'_>]) -> Shape {
    let mut composition = Composition::default();
    composition.apply(
        ast,
        styles
            .iter()
            .map(|prop| prop.clone_in(ast.allocator()))
            .collect(),
    );
    Shape(
        composition
            .into_residual()
            .into_iter()
            .map(|(key, _)| key)
            .collect(),
        coverage(styles),
    )
}

pub(super) fn selections<'a>(
    ast: &AstBuilder<'a>,
    styles: &[ExtractStyleProp<'a>],
    naming: Naming<'_>,
) -> Selections<'a> {
    let mut composition = Composition::default();
    composition.apply(
        ast,
        styles
            .iter()
            .map(|prop| prop.clone_in(ast.allocator()))
            .collect(),
    );
    let (keys, values) = composition
        .into_residual()
        .into_iter()
        .map(|(key, mut props)| {
            let classes = gen_class_names(ast, &mut props, None, naming.split_filename);
            (key, merge_expression_for_class_name(ast, classes))
        })
        .unzip();
    Selections {
        shape: Shape(keys, coverage(styles)),
        values,
    }
}

impl Shape {
    pub(super) fn union(&self, own: &Self) -> Self {
        let mut shape = self.clone();
        for key in &own.0 {
            if key.is_none() || !shape.0.contains(key) {
                shape.0.push(key.clone());
            }
        }
        shape.1 = self.1.union(&own.1);
        shape
    }

    pub(super) fn merge(&self, own: &Self, frame: &str, values: &str) -> String {
        let inherited = self.0.iter().enumerate().map(|(index, key)| {
            own.0
                .iter()
                .position(|other| key.is_some() && key == other)
                .map_or_else(
                    || format!("{frame}[0][{index}]"),
                    |later| format!("({values}[{later}]||{frame}[0][{index}])"),
                )
        });
        let added = own
            .0
            .iter()
            .enumerate()
            .filter(|(_, key)| key.is_none() || !self.0.contains(key))
            .map(|(index, _)| format!("{values}[{index}]"));
        format!("[{}]", inherited.chain(added).collect::<Vec<_>>().join(","))
    }
}
