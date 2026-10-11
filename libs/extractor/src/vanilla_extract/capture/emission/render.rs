use super::super::super::json_string;
use super::super::graph::{Kind, NodeId, Scalar, Shape, Value};
use super::EmitError;
use super::definitions::Definition;
use rustc_hash::FxHashMap;
use std::collections::BTreeSet;

pub(in crate::vanilla_extract::capture) struct Renderer<'a> {
    names: &'a FxHashMap<String, String>,
    pub(super) pools: Vec<String>,
    pooled: Vec<bool>,
    pub current: Vec<Option<Shape>>,
    pub(super) emitted: Vec<Option<Shape>>,
    pub(super) object: String,
    shadowed_undefined: bool,
}

fn fresh(base: &str, reserved: &mut BTreeSet<String>) -> String {
    let mut name = base.to_string();
    while reserved.contains(&name) {
        name.push('_');
    }
    reserved.insert(name.clone());
    name
}

impl<'a> Renderer<'a> {
    pub fn new(
        names: &'a FxHashMap<String, String>,
        reserved: &BTreeSet<String>,
        pooled: Vec<bool>,
    ) -> Self {
        let mut occupied = reserved.clone();
        occupied.extend(names.values().cloned());
        let object = fresh("__ve_object__", &mut occupied);
        let pools = pooled
            .iter()
            .enumerate()
            .map(|(index, _)| fresh(&format!("__ve_node_{index}__"), &mut occupied))
            .collect();
        Self {
            names,
            pools,
            current: vec![None; pooled.len()],
            emitted: vec![None; pooled.len()],
            pooled,
            object,
            shadowed_undefined: reserved.contains("undefined"),
        }
    }

    pub fn header(&self) -> String {
        let pools: Vec<_> = self
            .pools
            .iter()
            .zip(&self.pooled)
            .filter(|(_, pooled)| **pooled)
            .map(|(name, _)| name.as_str())
            .collect();
        if pools.is_empty() {
            String::new()
        } else {
            format!(
                "const {}=({{}}).constructor;let {};\n",
                self.object,
                pools.join(",")
            )
        }
    }

    pub fn observe(&mut self, shapes: &[(NodeId, Shape)]) {
        for (id, shape) in shapes {
            self.current[id.0] = Some(shape.clone());
        }
    }

    pub fn anchor(&mut self, anchors: &[(NodeId, String)]) -> String {
        let mut code = String::new();
        for (id, read) in anchors {
            code.push_str(&self.pools[id.0]);
            code.push('=');
            code.push_str(read);
            code.push(';');
            self.emitted[id.0].clone_from(&self.current[id.0]);
        }
        code
    }

    pub fn updates(&mut self) -> Result<Vec<String>, EmitError> {
        let mut code = Vec::new();
        for index in 0..self.current.len() {
            if let (Some(previous), Some(shape)) =
                (self.emitted[index].clone(), self.current[index].clone())
                && previous != shape
            {
                self.define(
                    Definition {
                        id: NodeId(index),
                        shape: &shape,
                        previous: Some(&previous),
                    },
                    &mut code,
                )?;
                self.emitted[index] = Some(shape);
            }
        }
        Ok(code)
    }

    pub fn value(&mut self, value: &Value, code: &mut Vec<String>) -> Result<String, EmitError> {
        match value {
            Value::Scalar(Scalar::Undefined) if self.shadowed_undefined => Ok("(void 0)".into()),
            Value::Scalar(value) => Ok(value.code(self.names)),
            Value::Node(id) => {
                let shape = self.current[id.0].clone().ok_or(EmitError::Unavailable)?;
                if self.pooled[id.0] {
                    if self.emitted[id.0].is_none() {
                        let initial = match shape.kind {
                            Kind::Record {
                                null_prototype: true,
                            } => format!("{}.create(null)", self.object),
                            Kind::Record {
                                null_prototype: false,
                            } => "{}".into(),
                            Kind::Array {
                                null_prototype: true,
                            } => format!("{}.setPrototypeOf([],null)", self.object),
                            Kind::Array {
                                null_prototype: false,
                            } => "[]".into(),
                        };
                        code.push(format!("{}={initial};", self.pools[id.0]));
                        self.emitted[id.0] = Some(shape.clone());
                        self.define(
                            Definition {
                                id: *id,
                                shape: &shape,
                                previous: None,
                            },
                            code,
                        )?;
                    }
                    Ok(self.pools[id.0].clone())
                } else {
                    let mut properties = Vec::new();
                    for property in &shape.properties {
                        let value = self.value(&property.value, code)?;
                        let key = if property.name == "__proto__" {
                            format!("[{}]", json_string(&property.name))
                        } else {
                            json_string(&property.name)
                        };
                        properties.push(format!("{key}: {value}"));
                    }
                    if properties.is_empty() {
                        Ok("{}".into())
                    } else {
                        Ok(format!("{{ {} }}", properties.join(", ")))
                    }
                }
            }
        }
    }
}
