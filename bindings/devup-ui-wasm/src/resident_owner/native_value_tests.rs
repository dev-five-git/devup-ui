use std::{
    cell::{Cell, RefCell},
    collections::BTreeMap,
    rc::Rc,
};

#[derive(Clone, Debug)]
pub(super) enum Value {
    Undefined,
    Bool(bool),
    Text(String),
    Object(Rc<Object>),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Undefined, Self::Undefined) => true,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::Text(a), Self::Text(b)) => a == b,
            (Self::Object(a), Self::Object(b)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct Property {
    pub value: Value,
    pub configurable: bool,
    pub writable: bool,
    pub enumerable: bool,
}

#[derive(Debug, Default)]
pub(super) struct Object {
    properties: RefCell<BTreeMap<String, Property>>,
    elements: RefCell<Vec<Value>>,
    descriptor: RefCell<Option<Property>>,
}

thread_local! {
    pub(super) static CREATED: Cell<usize> = const { Cell::new(0) };
}

impl Value {
    pub fn new() -> Self {
        Self::Object(Rc::new(Object::default()))
    }

    pub fn object(&self) -> &Rc<Object> {
        match self {
            Self::Object(object) => object,
            Self::Undefined | Self::Bool(_) | Self::Text(_) => panic!("object primitive contract"),
        }
    }

    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }
    pub fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }
    pub fn text(text: &str) -> Self {
        Self::Text(text.to_owned())
    }

    pub fn as_string(&self) -> Option<String> {
        match self {
            Self::Text(text) => Some(text.clone()),
            Self::Undefined | Self::Bool(_) | Self::Object(_) => None,
        }
    }

    pub fn get(&self, key: &str) -> Self {
        self.property(key).map_or(Self::Undefined, |p| p.value)
    }

    pub fn property(&self, key: &str) -> Option<Property> {
        self.object().properties.borrow().get(key).cloned()
    }

    pub fn set_property(&self, key: &str, value: Self) {
        Self::define(
            self.object(),
            &Self::text(key),
            Self::descriptor(&value).object(),
        );
    }

    pub fn array_from(value: &Self) -> Self {
        let copy = Self::new();
        *copy.object().elements.borrow_mut() = value.object().elements.borrow().clone();
        copy
    }

    pub fn push(&self, value: &Self) -> u32 {
        let mut elements = self.object().elements.borrow_mut();
        elements.push(value.clone());
        elements
            .len()
            .try_into()
            .unwrap_or_else(|error| panic!("native array fits u32: {error}"))
    }

    pub fn at(&self, index: u32) -> Self {
        self.object()
            .elements
            .borrow()
            .get(
                usize::try_from(index)
                    .unwrap_or_else(|error| panic!("native index fits usize: {error}")),
            )
            .cloned()
            .unwrap_or(Self::Undefined)
    }

    pub fn set(&self, index: u32, value: Self) {
        let index = usize::try_from(index)
            .unwrap_or_else(|error| panic!("native index fits usize: {error}"));
        let mut elements = self.object().elements.borrow_mut();
        let length = elements.len().max(index + 1);
        elements.resize(length, Self::Undefined);
        elements[index] = value;
    }

    pub fn of4(a: &Self, b: &Self, c: &Self, d: &Self) -> Self {
        let array = Self::new();
        *array.object().elements.borrow_mut() = vec![a.clone(), b.clone(), c.clone(), d.clone()];
        array
    }

    pub fn descriptor(value: &Self) -> Self {
        let descriptor = Self::new();
        *descriptor.object().descriptor.borrow_mut() = Some(Property {
            value: value.clone(),
            configurable: false,
            writable: false,
            enumerable: false,
        });
        descriptor
    }

    pub fn configurable(&self, value: bool) {
        self.object()
            .descriptor
            .borrow_mut()
            .as_mut()
            .unwrap_or_else(|| panic!("descriptor primitive receives a descriptor"))
            .configurable = value;
    }

    pub fn writable(&self, value: bool) {
        self.object()
            .descriptor
            .borrow_mut()
            .as_mut()
            .unwrap_or_else(|| panic!("descriptor primitive receives a descriptor"))
            .writable = value;
    }

    pub fn define(object: &Rc<Object>, key: &Self, descriptor: &Rc<Object>) -> Rc<Object> {
        let Self::Text(key) = key else {
            panic!("string key primitive contract")
        };
        object.properties.borrow_mut().insert(
            key.clone(),
            descriptor
                .descriptor
                .borrow()
                .clone()
                .unwrap_or_else(|| panic!("define primitive receives a descriptor")),
        );
        object.clone()
    }

    pub fn error(message: &str) -> Self {
        CREATED.set(CREATED.get() + 1);
        let error = Self::new();
        error.set_property("message", Self::text(message));
        error
    }

    pub fn name(&self, name: &str) {
        self.set_property("name", Self::text(name));
    }
}
