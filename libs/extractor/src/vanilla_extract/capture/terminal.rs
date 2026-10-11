use boa_engine::{Context, JsObject, JsValue, Source};
use rustc_hash::FxHashMap;

use super::emission::render::Renderer;
use super::graph::{Graph, GraphError, NodeId, Value};

#[derive(Default)]
pub(crate) struct Terminal {
    objects: Vec<(JsObject, JsObject)>,
    pub names: FxHashMap<String, String>,
}

impl Terminal {
    pub(crate) fn resolve(
        &mut self,
        value: &JsValue,
        context: &mut Context,
    ) -> boa_engine::JsResult<JsValue> {
        let value = self.read(value);
        if self.names.is_empty()
            || !value.is_object()
            || value.as_object().is_some_and(|object| object.is_callable())
        {
            return Ok(value);
        }
        let mut graph = Graph::default();
        match graph.value(&value, context) {
            Ok(_) => {}
            Err(GraphError::Unsupported(_)) => return Ok(value),
            Err(GraphError::Engine(error)) => return Err(error),
        }
        let requires = graph
            .nodes
            .iter()
            .filter_map(|node| node.shape.as_ref())
            .flat_map(|shape| &shape.properties)
            .any(|property| match &property.value {
                Value::Scalar(super::graph::Scalar::Text(text)) => {
                    super::super::replace_placeholders(text, |token, _| {
                        self.names.get(token).cloned()
                    })
                    .is_some()
                }
                Value::Scalar(
                    super::graph::Scalar::Number(_)
                    | super::graph::Scalar::Boolean(_)
                    | super::graph::Scalar::Null
                    | super::graph::Scalar::Undefined,
                )
                | Value::Node(_) => false,
            });
        if requires {
            self.materialize(std::slice::from_ref(&value), context)
                .map_err(|error| {
                    boa_engine::JsNativeError::typ().with_message(error.to_string())
                })?;
        }
        Ok(self.read(&value))
    }
    pub(crate) fn validate(values: &[JsValue], context: &mut Context) -> Result<(), GraphError> {
        let mut graph = Graph::default();
        for value in values {
            graph.value(value, context)?;
        }
        Ok(())
    }
    pub(crate) fn read(&self, value: &JsValue) -> JsValue {
        if let Some(text) = value.as_string() {
            let text = text.to_std_string_escaped();
            if let Some(replaced) =
                super::super::replace_placeholders(&text, |token, _| self.names.get(token).cloned())
            {
                return boa_engine::JsString::from(replaced).into();
            }
        }
        if let Some(object) = value.as_object() {
            let mut object = object;
            while let Some((_, terminal)) = self
                .objects
                .iter()
                .rev()
                .find(|(original, _)| JsObject::equals(original, &object))
            {
                object = terminal.clone();
            }
            return object.into();
        }
        value.clone()
    }

    /// Materialize terminal data through the capture graph, never author initializers.
    pub(crate) fn materialize(
        &mut self,
        values: &[JsValue],
        context: &mut Context,
    ) -> Result<(), GraphError> {
        let mut graph = Graph::default();
        for value in values {
            let value = self.read(value);
            graph.value(&value, context)?;
        }
        let names = self
            .names
            .iter()
            .map(|(id, value)| (id.clone(), super::super::json_string(value)))
            .collect();
        let reserved = std::collections::BTreeSet::new();
        let mut renderer = Renderer::new(&names, &reserved, vec![true; graph.nodes.len()]);
        renderer.observe(&graph.snapshots());
        let mut statements = Vec::new();
        let mut reads = Vec::new();
        for index in 0..graph.nodes.len() {
            reads.push(
                renderer
                    .value(&Value::Node(NodeId(index)), &mut statements)
                    .map_err(|_| GraphError::Unsupported("terminal graph is unavailable"))?,
            );
        }
        let code = format!(
            "(()=>{{{}{}return [{}];}})()",
            renderer.header(),
            statements.join(""),
            reads.join(",")
        );
        let array = context
            .eval(Source::from_bytes(code.as_bytes()))
            .map_err(GraphError::Engine)?;
        let array = array.as_object().ok_or(GraphError::Unsupported(
            "terminal graph did not return data",
        ))?;
        for (index, node) in graph.nodes.iter().enumerate() {
            let key = u32::try_from(index)
                .map_err(|_| GraphError::Unsupported("terminal graph is too large"))?;
            let terminal = array
                .get(key, context)
                .map_err(GraphError::Engine)?
                .as_object()
                .ok_or(GraphError::Unsupported(
                    "terminal node did not return an object",
                ))?;
            self.objects.push((node.object.clone(), terminal));
        }
        Ok(())
    }
}
