use super::{Graph, GraphError, Kind, Scalar, Value};
use boa_engine::{Context, JsObject, Source, property::Attribute};
use rstest::rstest;
use rustc_hash::FxHashMap;

#[rstest]
#[case("null", Scalar::Null, None)]
#[case("true", Scalar::Boolean(true), Some(true))]
#[case("false", Scalar::Boolean(false), Some(false))]
fn capture_graph_scalar_contract_when_value_is_null_or_boolean(
    #[case] source: &str,
    #[case] expected: Scalar,
    #[case] expected_boolean: Option<bool>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Given
    let mut context = Context::default();
    let mut graph = Graph::default();
    let value = context.eval(Source::from_bytes(source.as_bytes()))?;
    // When
    let captured = graph.value(&value, &mut context)?;
    // Then
    assert_eq!(captured, Value::Scalar(expected));
    assert_eq!(graph.nodes.len(), 0);
    let scalar = match captured {
        Value::Scalar(scalar) => scalar,
        Value::Node(_) => panic!("scalar capture returned a node"),
    };
    let code = scalar.code(&FxHashMap::default());
    let emitted = Context::default().eval(Source::from_bytes(code.as_bytes()))?;
    match expected_boolean {
        Some(expected) => assert_eq!(emitted.as_boolean(), Some(expected)),
        None => assert!(emitted.is_null()),
    }
    Ok(())
}

#[test]
fn capture_graph_rejects_array_when_prototype_is_custom() -> Result<(), Box<dyn std::error::Error>>
{
    // Given
    let source = "Object.setPrototypeOf([], {})";
    let mut context = Context::default();
    let mut graph = Graph::default();
    let value = context.eval(Source::from_bytes(source.as_bytes()))?;
    // When
    let result = graph.value(&value, &mut context);
    // Then
    let error = match result {
        Err(error) => error,
        Ok(value) => panic!("custom array prototype was captured: {value:?}"),
    };
    assert_eq!(error.to_string(), "custom array prototype");
    match error {
        GraphError::Unsupported(cause) => assert_eq!(cause, "custom array prototype"),
        GraphError::Engine(error) => return Err(error.into()),
    }
    assert_eq!(graph.nodes.len(), 0);
    Ok(())
}

#[test]
fn capture_graph_rejects_record_when_an_own_key_is_a_symbol()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = "({[Symbol('capture-key')]: 7})";
    let mut context = Context::default();
    let mut graph = Graph::default();
    let value = context.eval(Source::from_bytes(source.as_bytes()))?;
    // When
    let result = graph.value(&value, &mut context);
    // Then
    let error = match result {
        Err(error) => error,
        Ok(value) => panic!("symbol property was captured: {value:?}"),
    };
    assert_eq!(error.to_string(), "symbol property");
    match error {
        GraphError::Unsupported(cause) => assert_eq!(cause, "symbol property"),
        GraphError::Engine(error) => return Err(error.into()),
    }
    Ok(())
}

#[test]
fn capture_graph_keeps_array_shape_when_null_prototype_values_share_a_child()
-> Result<(), Box<dyn std::error::Error>> {
    // Given
    let source = concat!(
        "(()=>{const shared=",
        "{value:7};return Object.setPrototypeOf([shared,shared,null,true,false],null)})()"
    );
    let mut context = Context::default();
    let mut graph = Graph::default();
    let value = context.eval(Source::from_bytes(source.as_bytes()))?;
    // When
    let captured = graph.value(&value, &mut context)?;
    // Then
    let root_id = match captured {
        Value::Node(id) => id,
        Value::Scalar(scalar) => panic!("array capture returned a scalar: {scalar:?}"),
    };
    let root = graph
        .nodes
        .get(root_id.0)
        .ok_or_else(|| std::io::Error::other("missing root node"))?;
    let shape = root
        .shape
        .as_ref()
        .ok_or_else(|| std::io::Error::other("missing root shape"))?;
    assert_eq!(
        shape.kind,
        Kind::Array {
            null_prototype: true
        }
    );
    assert!(shape.extensible);
    assert!(!shape.plain());
    assert_eq!(
        shape
            .properties
            .iter()
            .map(|property| (property.name.as_str(), property.indexed))
            .collect::<Vec<_>>(),
        [
            ("0", true),
            ("1", true),
            ("2", true),
            ("3", true),
            ("4", true),
            ("length", false)
        ]
    );
    let [first, second, null, yes, no, length] = shape.properties.as_slice() else {
        panic!("array capture must contain five indices and length");
    };
    let child_id = match first.value {
        Value::Node(id) => id,
        Value::Scalar(ref scalar) => panic!("shared child returned a scalar: {scalar:?}"),
    };
    assert_eq!(second.value, Value::Node(child_id));
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(null.value, Value::Scalar(Scalar::Null));
    assert_eq!(yes.value, Value::Scalar(Scalar::Boolean(true)));
    assert_eq!(no.value, Value::Scalar(Scalar::Boolean(false)));
    assert_eq!(length.value, Value::Scalar(Scalar::Number("5".into())));
    assert_eq!(
        shape
            .properties
            .iter()
            .take(5)
            .map(|property| property.attributes)
            .collect::<Vec<_>>(),
        [Attribute::all(); 5]
    );
    assert_eq!(length.attributes, Attribute::WRITABLE);
    let original = value
        .as_object()
        .ok_or_else(|| std::io::Error::other("source is not an array object"))?;
    let original_child = original
        .get(0_u32, &mut context)?
        .as_object()
        .ok_or_else(|| std::io::Error::other("source child is not an object"))?;
    let child = graph
        .nodes
        .get(child_id.0)
        .ok_or_else(|| std::io::Error::other("missing child node"))?;
    assert!(JsObject::equals(&root.object, &original));
    assert!(JsObject::equals(&child.object, &original_child));
    assert!(!JsObject::equals(&root.object, &child.object));
    let child_shape = child
        .shape
        .as_ref()
        .ok_or_else(|| std::io::Error::other("missing child shape"))?;
    assert_eq!(
        child_shape.kind,
        Kind::Record {
            null_prototype: false
        }
    );
    assert!(child_shape.extensible);
    assert!(child_shape.plain());
    let [property] = child_shape.properties.as_slice() else {
        panic!("shared child must contain one property");
    };
    assert_eq!(property.name, "value");
    assert!(!property.indexed);
    assert_eq!(property.value, Value::Scalar(Scalar::Number("7".into())));
    assert_eq!(property.attributes, Attribute::all());
    Ok(())
}
