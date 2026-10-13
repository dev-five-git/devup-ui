use std::collections::BTreeSet;
use std::vec::IntoIter;

use boa_engine::{
    Context, JsNativeError, JsObject, JsResult, JsValue, builtins::object::OrdinaryObject,
    js_string, object::ObjectInitializer, property::PropertyKey,
};

use super::own_keys;

type Leaf<'a> = dyn FnMut(&JsValue, &[String], &mut Context) -> JsResult<JsValue> + 'a;

/// Live `for...in` keys, snapshotting each object's keys only when reached.
pub(super) struct EnumerableKeys {
    current: Option<JsObject>,
    remaining: Option<IntoIter<(PropertyKey, String)>>,
    seen: BTreeSet<String>,
}

impl EnumerableKeys {
    pub(super) fn new(tokens: &JsValue, context: &mut Context) -> JsResult<Self> {
        let current = if tokens.is_null_or_undefined() {
            None
        } else {
            Some(tokens.to_object(context)?)
        };
        Ok(Self {
            current,
            remaining: None,
            seen: BTreeSet::new(),
        })
    }

    pub(super) fn next(
        &mut self,
        context: &mut Context,
    ) -> JsResult<Option<(PropertyKey, String)>> {
        while let Some(object) = &self.current {
            if self.remaining.is_none() {
                self.remaining = Some(own_keys(object, context)?.into_iter());
            }
            while let Some((key, name)) = self.remaining.as_mut().and_then(Iterator::next) {
                if self.seen.contains(&name) {
                    continue;
                }
                let descriptor = OrdinaryObject::get_own_property_descriptor(
                    &JsValue::undefined(),
                    &[object.clone().into(), (&key).into()],
                    context,
                )?;
                let Some(descriptor) = descriptor.as_object() else {
                    continue;
                };
                self.seen.insert(name.clone());
                if descriptor
                    .get(js_string!("enumerable"), context)?
                    .to_boolean()
                {
                    return Ok(Some((key, name)));
                }
            }
            self.current = OrdinaryObject::get_prototype_of(
                &JsValue::undefined(),
                &[object.clone().into()],
                context,
            )?
            .as_object();
            self.remaining = None;
        }
        Ok(None)
    }
}

/// Replace upstream token leaves; arrays, functions and booleans are skipped.
pub(super) fn walk_object(
    tokens: &JsValue,
    context: &mut Context,
    path: &mut Vec<String>,
    leaf: &mut Leaf<'_>,
) -> JsResult<JsValue> {
    TokenWalk {
        path,
        leaf,
        ancestry: Vec::new(),
    }
    .walk(tokens, context)
}

struct TokenWalk<'walk, 'leaf> {
    path: &'walk mut Vec<String>,
    leaf: &'walk mut Leaf<'leaf>,
    ancestry: Vec<JsObject>,
}

impl TokenWalk<'_, '_> {
    fn walk(&mut self, tokens: &JsValue, context: &mut Context) -> JsResult<JsValue> {
        let object = tokens.as_object();
        if let Some(object) = &object {
            if self
                .ancestry
                .iter()
                .any(|ancestor| JsObject::equals(ancestor, object))
            {
                return Err(JsNativeError::error()
                    .with_message(format!(
                        "Cyclic theme token object at \"{}\" cannot form a finite contract. Fix: supply an acyclic finite token object",
                        self.path.join(".")
                    ))
                    .into());
            }
            self.ancestry.push(object.clone());
        }
        let result = (|| {
            let walked = ObjectInitializer::new(context).build();
            let mut keys = EnumerableKeys::new(tokens, context)?;
            while let Some((key, name)) = keys.next(context)? {
                let value = tokens.to_object(context)?.get(key, context)?;
                self.path.push(name.clone());
                let mapped =
                    if value.is_string() || value.is_number() || value.is_null_or_undefined() {
                        (self.leaf)(&value, self.path, context).map(Some)
                    } else if value
                        .as_object()
                        .is_some_and(|object| !object.is_array() && !object.is_callable())
                    {
                        self.walk(&value, context).map(Some)
                    } else {
                        Ok(None)
                    };
                self.path.pop();
                if let Some(mapped) = mapped? {
                    walked.set(js_string!(name), mapped, false, context)?;
                }
            }
            Ok(walked.into())
        })();
        if object.is_some() {
            self.ancestry.pop();
        }
        result
    }
}
