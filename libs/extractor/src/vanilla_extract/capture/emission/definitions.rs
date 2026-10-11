use super::super::super::json_string;
use super::super::graph::{Kind, NodeId, Shape};
use super::EmitError;
use super::render::Renderer;

pub(super) struct Definition<'a> {
    pub id: NodeId,
    pub shape: &'a Shape,
    pub previous: Option<&'a Shape>,
}

impl Renderer<'_> {
    pub(super) fn define(
        &mut self,
        definition: Definition<'_>,
        code: &mut Vec<String>,
    ) -> Result<(), EmitError> {
        let Definition {
            id,
            shape,
            previous,
        } = definition;
        let name = self.pools[id.0].clone();
        if let Some(previous) = previous {
            let ordered = |shape: &Shape| match shape.kind {
                Kind::Record { .. } => shape
                    .properties
                    .iter()
                    .filter(|property| !property.indexed)
                    .map(|property| property.name.clone())
                    .collect::<Vec<_>>(),
                Kind::Array { .. } => Vec::new(),
            };
            let old_order = ordered(previous);
            let new_order = ordered(shape);
            let mut retained = Vec::new();
            let mut cursor = 0;
            for name in &new_order {
                let Some(relative) = old_order[cursor..].iter().position(|old| old == name) else {
                    break;
                };
                cursor += relative + 1;
                retained.push(name.clone());
            }
            for property in &previous.properties {
                let reordered = old_order != new_order
                    && old_order.contains(&property.name)
                    && !retained.contains(&property.name);
                if reordered
                    || !shape
                        .properties
                        .iter()
                        .any(|next| next.name == property.name)
                {
                    if !property.attributes.configurable() {
                        return Err(EmitError::ImmutableOrder);
                    }
                    code.push(format!("delete {name}[{}];", json_string(&property.name)));
                }
            }
            if previous.kind != shape.kind {
                let prototype = match shape.kind {
                    Kind::Record {
                        null_prototype: true,
                    }
                    | Kind::Array {
                        null_prototype: true,
                    } => "null".into(),
                    Kind::Record {
                        null_prototype: false,
                    } => format!("{}.prototype", self.object),
                    Kind::Array {
                        null_prototype: false,
                    } => "[].constructor.prototype".into(),
                };
                code.push(format!(
                    "{}.setPrototypeOf({name},{prototype});",
                    self.object
                ));
            }
        }
        for property in &shape.properties {
            if previous.is_some_and(|previous| {
                previous.properties == shape.properties
                    || previous.properties.iter().any(|old| old == property)
                        && previous
                            .properties
                            .iter()
                            .map(|old| &old.name)
                            .eq(shape.properties.iter().map(|next| &next.name))
            }) {
                continue;
            }
            let value = self.value(&property.value, code)?;
            code.push(format!("{}.defineProperty({name},{},{{value:{value},writable:{},enumerable:{},configurable:{}}});", self.object, json_string(&property.name), property.attributes.writable(), property.attributes.enumerable(), property.attributes.configurable()));
        }
        if !shape.extensible && previous.is_none_or(|previous| previous.extensible) {
            code.push(format!("{}.preventExtensions({name});", self.object));
        }
        Ok(())
    }
}
