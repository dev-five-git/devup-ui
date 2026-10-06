use super::graph::{Graph, NodeId, Shape, Value};
use boa_engine::{Context, JsArgs, JsResult, JsString, JsValue, NativeFunction, Source};
use oxc_span::Span;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ObservationKind {
    Input { declarator: bool },
    Before,
    After,
}

#[derive(Clone)]
pub(crate) struct Observation {
    pub span: Span,
    pub place: String,
    pub kind: ObservationKind,
    pub reads: Vec<String>,
    pub mutations: Vec<(String, String)>,
}

pub(super) struct Recorded {
    pub site: Observation,
    pub shapes: Vec<(NodeId, Shape)>,
    pub anchors: Vec<(NodeId, String)>,
    pub bindings: Vec<(String, Value)>,
}

pub(super) struct State {
    pub graph: Graph,
    pub recorded: Vec<Recorded>,
    plan: Vec<Observation>,
    pub failure: Option<String>,
    unsupported: std::collections::BTreeSet<String>,
    bindings: std::collections::BTreeMap<String, Value>,
}

pub(super) fn prepare(context: &mut Context, selected: &super::Selected<'_>) -> JsResult<()> {
    context.insert_data(State {
        graph: Graph::default(),
        recorded: Vec::new(),
        plan: selected.observations.to_vec(),
        failure: None,
        unsupported: std::collections::BTreeSet::new(),
        bindings: std::collections::BTreeMap::new(),
    });
    context.register_global_builtin_callable(
        JsString::from(selected.observer),
        1,
        NativeFunction::from_fn_ptr(observe),
    )
}

fn observe(_: &JsValue, args: &[JsValue], context: &mut Context) -> JsResult<JsValue> {
    let index = usize::try_from(args.get_or_undefined(0).to_u32(context)?)
        .map_err(|error| boa_engine::JsNativeError::range().with_message(error.to_string()))?;
    let mut state = context.remove_data::<State>().ok_or_else(|| {
        boa_engine::JsNativeError::typ().with_message("missing capture observations")
    })?;
    let result = record(&mut state, index, args, context);
    context.insert_data(*state);
    result.map(|()| JsValue::undefined())
}

fn record(
    state: &mut State,
    index: usize,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<()> {
    let site = state.plan.get(index).cloned().ok_or_else(|| {
        boa_engine::JsNativeError::range().with_message("unknown capture observation")
    })?;
    let mut anchors = Vec::new();
    for (read, place) in &site.mutations {
        if state.unsupported.contains(read) {
            state.failure = Some(format!(
                "{place}: retained native input `{read}` cannot capture its mutation exactly: callable, accessor or proxy state has no exact data-only reconstruction. Fix: keep mutable data in a plain record or inside the consumed initializer"
            ));
        }
    }
    for (value, read) in args.iter().skip(1).zip(&site.reads) {
        let input = matches!(site.kind, ObservationKind::Input { .. });
        if !input && !Graph::data_object(value) {
            continue;
        }
        match state.graph.value(value, context) {
            Ok(captured) => {
                if input {
                    state.bindings.insert(read.clone(), captured.clone());
                    paths(&state.graph, &captured, read, &mut anchors);
                }
            }
            Err(cause) => {
                if input {
                    state.unsupported.insert(read.clone());
                } else {
                    state.failure = Some(format!(
                        "{}: native result cannot be captured exactly: {cause}. Fix: return literals, arrays or plain records instead of functions, cyclic or exotic values",
                        site.place
                    ));
                }
            }
        }
    }
    let objects: Vec<_> = state
        .graph
        .nodes
        .iter()
        .filter(|node| node.shape.is_some())
        .map(|node| node.object.clone())
        .collect();
    for object in objects {
        if let Err(cause) = state.graph.value(&object.into(), context) {
            state.failure = Some(format!(
                "{}: retained native input cannot be captured exactly: {cause}. Fix: keep exact data-only mutations inside the selected initializer",
                site.place
            ));
        }
    }
    let mut bindings = Vec::new();
    for (read, previous) in &mut state.bindings {
        let value = context.eval(Source::from_bytes(read.as_bytes()))?;
        let value = match state.graph.value(&value, context) {
            Ok(value) => value,
            Err(cause) => {
                state.failure = Some(format!(
                    "{}: retained binding `{read}` cannot capture its state exactly: {cause}. Fix: keep data-only state in the consumed initializer",
                    site.place
                ));
                continue;
            }
        };
        if *previous != value {
            bindings.push((read.clone(), value.clone()));
        }
        *previous = value;
    }
    state.recorded.push(Recorded {
        site,
        shapes: state.graph.snapshots(),
        anchors,
        bindings,
    });
    Ok(())
}

fn paths(graph: &Graph, value: &Value, read: &str, anchors: &mut Vec<(NodeId, String)>) {
    let Value::Node(id) = value else { return };
    if anchors.iter().any(|(known, _)| known == id) {
        return;
    }
    anchors.push((*id, read.to_string()));
    if let Some(shape) = &graph.nodes[id.0].shape {
        for property in &shape.properties {
            let child = format!("{read}[{}]", super::super::json_string(&property.name));
            paths(graph, &property.value, &child, anchors);
        }
    }
}
