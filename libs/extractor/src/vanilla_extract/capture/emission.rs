use super::{
    Capture, FinalizeError,
    graph::{Shape, Value},
    observations::{ObservationKind, State},
};
use oxc_span::Span;
use rustc_hash::FxHashMap;
use std::collections::BTreeSet;

mod definitions;
pub(super) mod render;
use render::Renderer;

#[derive(Debug)]
pub(super) enum EmitError {
    Unavailable,
    ImmutableOrder,
}

impl std::fmt::Display for EmitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Unavailable => "data was not available at its original capture site",
            Self::ImmutableOrder => "nonconfigurable property cannot be reordered",
        })
    }
}

impl std::error::Error for EmitError {}

#[derive(Default)]
pub(crate) struct Emission {
    pub header: String,
    pub inputs: Vec<(Span, bool, String)>,
    pub effects: Vec<(Span, String)>,
}

#[derive(Default)]
pub(in crate::vanilla_extract) struct Finished {
    pub captures: Vec<String>,
    pub emission: Emission,
    pub readback: Vec<(Span, String)>,
}

pub(super) struct Inputs<'a> {
    pub roots: &'a [Value],
    pub captures: &'a [Capture],
    pub names: &'a FxHashMap<String, String>,
    pub reserved: &'a BTreeSet<String>,
}

pub(super) fn finish(
    state: &State,
    input: Inputs<'_>,
    snapshot_only: &[Span],
) -> Result<Finished, FinalizeError> {
    let mut uses = vec![0usize; state.graph.nodes.len()];
    for (value, capture) in input.roots.iter().zip(input.captures) {
        if !snapshot_only.contains(&capture.root) {
            count(value, &mut uses);
        }
    }
    for node in &state.graph.nodes {
        if let Some(shape) = &node.shape {
            for property in &shape.properties {
                count(&property.value, &mut uses);
            }
        }
    }
    let mut pooled: Vec<_> = state
        .graph
        .nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            uses[index] > 1 || node.shape.as_ref().is_some_and(|shape| !shape.plain())
        })
        .collect();
    let mut first: Vec<Option<Shape>> = vec![None; pooled.len()];
    for event in state
        .recorded
        .iter()
        .filter(|event| !snapshot_only.contains(&event.site.span))
    {
        for (id, _) in &event.anchors {
            pooled[id.0] = true;
        }
        for (_, value) in &event.bindings {
            match value {
                Value::Node(id) => pooled[id.0] = true,
                Value::Scalar(_) => {}
            }
        }
        for (id, shape) in &event.shapes {
            if first[id.0].as_ref().is_some_and(|first| first != shape) {
                pooled[id.0] = true;
            }
            first[id.0].get_or_insert_with(|| shape.clone());
        }
    }
    let mut renderer = Renderer::new(input.names, input.reserved, pooled);
    let mut emission = Emission::default();
    let mut captures = vec![String::new(); input.roots.len()];
    for event in state
        .recorded
        .iter()
        .filter(|event| !snapshot_only.contains(&event.site.span))
    {
        renderer.observe(&event.shapes);
        match event.site.kind {
            ObservationKind::Input { declarator } => {
                let code = renderer.anchor(&event.anchors);
                if !code.is_empty() {
                    emission.inputs.push((event.site.span, declarator, code));
                }
            }
            ObservationKind::Before => {}
            ObservationKind::After => {
                let mut statements = renderer.updates().map_err(|cause|
                    FinalizeError::Located(format!("{}: native data update cannot be captured exactly: {cause}. Fix: retain exact data at its original initialization site", event.site.place)))?;
                for (read, value) in &event.bindings {
                    let value = renderer.value(value, &mut statements).map_err(|cause|
                        FinalizeError::Located(format!("{}: native binding update cannot be captured exactly: {cause}. Fix: use exact data-only state", event.site.place)))?;
                    statements.push(format!("{read}={value};"));
                }
                for (index, capture) in input
                    .captures
                    .iter()
                    .enumerate()
                    .filter(|(_, capture)| capture.root == event.site.span)
                {
                    let value = renderer.value(&input.roots[index], &mut statements).map_err(|cause|
                        FinalizeError::Located(format!("{}: native result cannot be captured exactly: {cause}. Fix: keep exact data construction at its original initializer", capture.place)))?;
                    captures[index] = value;
                }
                if !statements.is_empty() {
                    emission
                        .effects
                        .push((event.site.span, statements.join("")));
                }
            }
        }
    }
    for (index, value) in input.roots.iter().enumerate() {
        if snapshot_only.contains(&input.captures[index].root) {
            continue;
        }
        if captures[index].is_empty() {
            let mut statements = Vec::new();
            let value = renderer.value(value, &mut statements).map_err(|cause|
                FinalizeError::Located(format!("{}: native result cannot be captured exactly: {cause}. Fix: use exact data-only initialization", input.captures[index].place)))?;
            captures[index] = value;
            if !statements.is_empty() {
                emission
                    .effects
                    .push((input.captures[index].root, statements.join("")));
            }
        }
    }
    emission.header = renderer.header();
    Ok(Finished {
        captures,
        emission,
        readback: Vec::new(),
    })
}

pub(super) fn readback(
    state: &State,
    input: Inputs<'_>,
    slots: &[Span],
) -> Result<Vec<(Span, String)>, FinalizeError> {
    let roots: Vec<_> = input
        .captures
        .iter()
        .zip(input.roots)
        .filter(|(capture, _)| slots.contains(&capture.root))
        .map(|(capture, value)| {
            let site = state.recorded.iter().find(|event| event.site.span == capture.root && event.site.kind == ObservationKind::After)
                .ok_or_else(|| FinalizeError::Located(format!("{}: consumer read has no original-site snapshot. Fix: report this extraction error", capture.place)))?;
            Ok((capture, value, site))
        }).collect::<Result<Vec<_>, FinalizeError>>()?;
    let mut uses = vec![0usize; state.graph.nodes.len()];
    let mut pooled = vec![false; state.graph.nodes.len()];
    let mut pending: Vec<_> = roots
        .iter()
        .map(|(capture, value, site)| (*value, *site, capture.root.start))
        .collect();
    let mut visited = BTreeSet::new();
    while let Some((value, site, root)) = pending.pop() {
        count(value, &mut uses);
        if let Value::Node(id) = value
            && visited.insert((root, id.0))
            && let Some((_, shape)) = site.shapes.iter().find(|(node, _)| node == id)
        {
            pooled[id.0] |= !shape.plain();
            pending.extend(
                shape
                    .properties
                    .iter()
                    .map(|property| (&property.value, site, root)),
            );
        }
    }
    for (pool, count) in pooled.iter_mut().zip(uses) {
        *pool |= count > 1;
    }
    if pooled.iter().any(|pooled| *pooled)
        && let Some((capture, _, _)) = roots
            .iter()
            .find(|(_, value, _)| matches!(value, Value::Node(_)))
    {
        return Err(FinalizeError::Located(format!(
            "{}: consumer result cannot be represented exactly as a styling literal. Fix: observe a scalar leaf or compute the observation inside the selected helper",
            capture.place
        )));
    }
    let mut reads = Vec::new();
    for (capture, value, site) in roots {
        let mut renderer = Renderer::new(input.names, input.reserved, pooled.clone());
        renderer.observe(&site.shapes);
        let mut statements = Vec::new();
        let expression = renderer.value(value, &mut statements).map_err(|cause|
            FinalizeError::Located(format!("{}: consumer result cannot be captured exactly: {cause}. Fix: return exact data-only values", capture.place)))?;
        if !statements.is_empty() || !renderer.header().is_empty() {
            return Err(FinalizeError::Located(format!(
                "{}: consumer result cannot be represented exactly as a styling literal. Fix: observe a scalar leaf or compute the observation inside the selected helper",
                capture.place
            )));
        }
        reads.push((capture.root, expression));
    }
    Ok(reads)
}

const fn count(value: &Value, uses: &mut [usize]) {
    match value {
        Value::Node(id) => uses[id.0] += 1,
        Value::Scalar(_) => {}
    }
}
