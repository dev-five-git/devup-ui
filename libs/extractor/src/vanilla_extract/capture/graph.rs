use super::super::is_plain_object;
use boa_engine::{
    Context, JsObject, JsValue,
    property::{Attribute, PropertyKey},
};

mod scalar;
mod shape;
pub(super) use scalar::Scalar;
pub(super) use shape::{Kind, Property, Shape};

#[derive(Debug)]
pub(super) enum GraphError {
    Unsupported(&'static str),
    Engine(boa_engine::JsError),
}

impl From<&'static str> for GraphError {
    fn from(cause: &'static str) -> Self {
        Self::Unsupported(cause)
    }
}

impl std::fmt::Display for GraphError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unsupported(cause) => formatter.write_str(cause),
            Self::Engine(error) => std::fmt::Display::fmt(error, formatter),
        }
    }
}

impl std::error::Error for GraphError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct NodeId(pub usize);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Value {
    Scalar(Scalar),
    Node(NodeId),
}

pub(super) struct Node {
    pub object: JsObject,
    pub shape: Option<Shape>,
}

#[derive(Default)]
pub(super) struct Graph {
    pub nodes: Vec<Node>,
    active: Vec<NodeId>,
}

impl Graph {
    pub fn value(&mut self, value: &JsValue, context: &mut Context) -> Result<Value, GraphError> {
        if let Some(scalar) = Scalar::read(value) {
            return Ok(Value::Scalar(scalar));
        }
        let object = value.as_object().ok_or("symbol or unsupported scalar")?;
        if object.is_callable() || object.is::<boa_engine::builtins::proxy::Proxy>() {
            return Err("function or proxy".into());
        }
        let kind = if object.is_array() {
            let prototype = object.prototype();
            if prototype.as_ref().is_some_and(|prototype| {
                !JsObject::equals(
                    prototype,
                    &context.intrinsics().constructors().array().prototype(),
                )
            }) {
                return Err("custom array prototype".into());
            }
            Kind::Array {
                null_prototype: prototype.is_none(),
            }
        } else if object.is::<boa_engine::builtins::object::OrdinaryObject>()
            && is_plain_object(&object, context)
        {
            Kind::Record {
                null_prototype: object.prototype().is_none(),
            }
        } else {
            return Err("exotic object".into());
        };
        let id = self
            .nodes
            .iter()
            .position(|node| JsObject::equals(&node.object, &object))
            .map_or_else(
                || {
                    let id = NodeId(self.nodes.len());
                    self.nodes.push(Node {
                        object: object.clone(),
                        shape: None,
                    });
                    id
                },
                NodeId,
            );
        if self.active.contains(&id) {
            return Err("cyclic object".into());
        }
        self.active.push(id);
        let shape = self.shape(&object, kind, context);
        self.active.pop();
        self.nodes[id.0].shape = Some(shape?);
        Ok(Value::Node(id))
    }

    fn shape(
        &mut self,
        object: &JsObject,
        kind: Kind,
        context: &mut Context,
    ) -> Result<Shape, GraphError> {
        let keys = object
            .own_property_keys(context)
            .map_err(GraphError::Engine)?;
        let mut properties = Vec::new();
        for key in keys {
            let name = match &key {
                PropertyKey::Symbol(_) => return Err("symbol property".into()),
                PropertyKey::Index(index) => index.get().to_string(),
                PropertyKey::String(name) => name.to_std_string_escaped(),
            };
            if matches!(kind, Kind::Array { .. })
                && matches!(&key, PropertyKey::String(_))
                && name != "length"
            {
                return Err("named array extra".into());
            }
            let descriptor = object
                .borrow()
                .properties()
                .get(&key)
                .ok_or("missing own descriptor")?;
            let value = descriptor.value().cloned().ok_or("accessor property")?;
            let mut attributes = Attribute::empty();
            attributes.set(Attribute::WRITABLE, descriptor.writable().unwrap_or(false));
            attributes.set(
                Attribute::ENUMERABLE,
                descriptor.enumerable().unwrap_or(false),
            );
            attributes.set(
                Attribute::CONFIGURABLE,
                descriptor.configurable().unwrap_or(false),
            );
            let property = Property {
                name,
                indexed: matches!(key, PropertyKey::Index(_)),
                value: self.value(&value, context)?,
                attributes,
            };
            properties.push(property);
        }
        Ok(Shape {
            kind,
            properties,
            extensible: object.is_extensible(context).map_err(GraphError::Engine)?,
        })
    }

    pub fn snapshots(&self) -> Vec<(NodeId, Shape)> {
        self.nodes
            .iter()
            .enumerate()
            .filter_map(|(index, node)| node.shape.clone().map(|shape| (NodeId(index), shape)))
            .collect()
    }

    pub fn data_object(value: &JsValue) -> bool {
        value
            .as_object()
            .is_some_and(|object| !object.is_callable())
    }
}
