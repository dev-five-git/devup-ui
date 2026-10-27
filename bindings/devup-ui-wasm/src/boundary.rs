use bimap::BiHashMap;
use serde::de::{DeserializeOwned, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
use std::marker::PhantomData;
use wasm_bindgen::prelude::*;

/// File slots grow only from decoded entries, never a JavaScript length hint.
pub(crate) struct FileMap(pub(crate) BiHashMap<String, usize>);

impl<'de> Deserialize<'de> for FileMap {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct FileMapVisitor;

        impl<'de> Visitor<'de> for FileMapVisitor {
            type Value = FileMap;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a map")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut entries: A) -> Result<FileMap, A::Error> {
                let mut map = BiHashMap::new();
                while let Some((file, index)) = entries.next_entry::<String, usize>()? {
                    map.insert(file, index);
                }
                Ok(FileMap(map))
            }
        }

        deserializer.deserialize_map(FileMapVisitor)
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = __devupBoundary)]
    fn begin(slot: u32);
    #[wasm_bindgen(js_namespace = __devupBoundary)]
    fn end();
    #[wasm_bindgen(js_namespace = __devupBoundary)]
    fn error(value: JsValue) -> JsValue;
    #[wasm_bindgen(js_namespace = __devupBoundary)]
    pub(crate) fn fail(slot: u32) -> JsValue;
}

/// Function-item lifecycle hooks stay statically dispatched, including on WASM.
pub(crate) struct Decoder<D, J, L> {
    lifecycle: L,
    types: PhantomData<fn() -> (D, J)>,
}

impl<D, J, B, E, F> Decoder<D, J, (B, E, F)>
where
    B: Fn(u32),
    E: Fn(),
    F: Fn(J) -> J,
{
    pub(crate) fn decode<'de, T: DeserializeOwned, I>(&self, value: I, slot: u32) -> Result<T, J>
    where
        D: Deserializer<'de> + From<I>,
        J: From<D::Error>,
    {
        (self.lifecycle.0)(slot);
        match T::deserialize(D::from(value)) {
            Ok(value) => {
                (self.lifecycle.1)();
                Ok(value)
            }
            Err(cause) => Err((self.lifecycle.2)(J::from(cause))),
        }
    }
}

type WasmDecoder<B, E, F> = Decoder<serde_wasm_bindgen::Deserializer, JsValue, (B, E, F)>;

pub(crate) const fn wasm() -> WasmDecoder<impl Fn(u32), impl Fn(), impl Fn(JsValue) -> JsValue> {
    Decoder {
        lifecycle: (begin, end, error),
        types: PhantomData,
    }
}

#[cfg(test)]
mod tests {
    use super::{Decoder, FileMap, wasm};
    use serde::Deserialize;
    use serde::de::value::{Error, MapDeserializer};
    use std::cell::RefCell;
    use std::marker::PhantomData;

    std::thread_local! {
        static EVENTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    }

    struct Original(serde_json::Value);

    impl From<Original> for serde_json::Value {
        fn from(value: Original) -> Self {
            EVENTS.with_borrow_mut(|events| events.push("construct".into()));
            value.0
        }
    }

    struct Observed<T>(T);

    impl<'de, T: Deserialize<'de>> Deserialize<'de> for Observed<T> {
        fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
            EVENTS.with_borrow_mut(|events| events.push("deserialize".into()));
            T::deserialize(deserializer).map(Self)
        }
    }

    #[derive(Debug)]
    struct Cause(serde_json::Error);

    impl From<serde_json::Error> for Cause {
        fn from(cause: serde_json::Error) -> Self {
            EVENTS.with_borrow_mut(|events| events.push("convert".into()));
            Self(cause)
        }
    }

    type NativeDecoder<B, E, F> = Decoder<serde_json::Value, Cause, (B, E, F)>;

    fn native() -> NativeDecoder<impl Fn(u32), impl Fn(), impl Fn(Cause) -> Cause> {
        Decoder {
            lifecycle: (
                |slot| EVENTS.with_borrow_mut(|events| *events = vec![format!("begin:{slot}")]),
                || EVENTS.with_borrow_mut(|events| events.push("end".into())),
                |cause| {
                    EVENTS.with_borrow_mut(|events| events.push("error".into()));
                    cause
                },
            ),
            types: PhantomData,
        }
    }

    #[test]
    fn ends_after_real_file_map_decode_when_input_is_valid() -> Result<(), serde_json::Error> {
        // Given the production wiring can be selected without invoking JS imports.
        assert_eq!(std::mem::size_of_val(&wasm()), 0);
        let source = Original(serde_json::json!({"a": 7, "b": 9}));
        // When the common body constructs a real Serde deserializer and decodes FileMap.
        let Observed(FileMap(map)) = native().decode(source, 7).map_err(|cause| cause.0)?;
        // Then decoded entries and the complete success-only lifecycle survive.
        assert_eq!(map.get_by_left("a"), Some(&7));
        assert_eq!(map.get_by_left("b"), Some(&9));
        EVENTS.with_borrow(|events| {
            assert_eq!(*events, ["begin:7", "construct", "deserialize", "end"]);
        });
        Ok(())
    }

    #[test]
    fn decodes_typed_rows_when_a_different_target_type_is_selected() -> Result<(), serde_json::Error>
    {
        // Given a reservation-shaped input rather than a file map.
        let source = Original(serde_json::json!([["src/a.tsx", "source", "df"]]));
        // When the common body deserializes a different owned target type.
        let Observed(rows): Observed<Vec<Vec<String>>> =
            native().decode(source, 0).map_err(|cause| cause.0)?;
        // Then the real nested rows, not an untyped pass-through, are returned.
        assert_eq!(rows, [["src/a.tsx", "source", "df"]]);
        EVENTS.with_borrow(|events| {
            assert_eq!(*events, ["begin:0", "construct", "deserialize", "end"]);
        });
        Ok(())
    }

    #[test]
    fn converts_original_cause_once_when_file_map_input_is_invalid() {
        // Given invalid map shapes and values, using real Serde Value deserializers.
        for (source, message) in [
            (
                serde_json::json!(false),
                "invalid type: boolean `false`, expected a map",
            ),
            (
                serde_json::json!({"a": false}),
                "invalid type: boolean `false`, expected usize",
            ),
            (
                serde_json::json!({"a": -1}),
                "invalid value: integer `-1`, expected usize",
            ),
        ] {
            // When the same production lifecycle encounters a decoding error.
            let result: Result<Observed<FileMap>, Cause> = native().decode(Original(source), 3);
            // Then the original cause reaches conversion/error once, with no success end.
            let Err(cause) = result else {
                panic!("invalid input decoded successfully");
            };
            assert_eq!(cause.0.to_string(), message);
            EVENTS.with_borrow(|events| {
                assert_eq!(
                    *events,
                    ["begin:3", "construct", "deserialize", "convert", "error"]
                );
            });
        }
    }

    struct HugeHint<I>(I);

    impl<I: Iterator> Iterator for HugeHint<I> {
        type Item = I::Item;

        fn next(&mut self) -> Option<Self::Item> {
            self.0.next()
        }

        fn size_hint(&self) -> (usize, Option<usize>) {
            (usize::MAX, Some(usize::MAX))
        }
    }

    #[test]
    fn decodes_entries_when_allocation_hint_is_huge() -> Result<(), Error> {
        // Given an untrusted exact hint and a single real entry.
        let entries = HugeHint([("a", 7_usize)].into_iter());
        assert_eq!(entries.size_hint(), (usize::MAX, Some(usize::MAX)));
        let deserializer = MapDeserializer::<_, Error>::new(entries);
        // When decoding through the concrete file-map Deserialize implementation.
        let FileMap(map) = FileMap::deserialize(deserializer)?;
        // Then the real entry is retained without reserving the advertised capacity.
        assert_eq!(map.len(), 1);
        assert_eq!(map.get_by_left("a"), Some(&7));
        Ok(())
    }

    #[test]
    fn preserves_overwrite_order_when_keys_and_values_repeat() -> Result<(), serde_json::Error> {
        // Given successive key and value collisions (buffering by key resurrects a).
        let source = r#"{"a":1,"b":1,"b":2,"c":2,"c":3}"#;
        // When consuming the pairs in source order.
        let FileMap(map) = serde_json::from_str(source)?;
        // Then only the last surviving bimap pair remains.
        assert_eq!(map.len(), 1);
        assert_eq!(map.get_by_left("c"), Some(&3));
        Ok(())
    }

    #[test]
    fn rejects_invalid_shapes_and_values_when_deserializing() {
        // Given a non-map and invalid typed file indices.
        for (source, message) in [
            (
                "false",
                "invalid type: boolean `false`, expected a map at line 1 column 5",
            ),
            (
                r#"{"a":false}"#,
                "invalid type: boolean `false`, expected usize at line 1 column 10",
            ),
            (
                r#"{"a":-1}"#,
                "invalid value: integer `-1`, expected usize at line 1 column 7",
            ),
        ] {
            // When the same concrete decoder receives invalid input.
            let result = serde_json::from_str::<FileMap>(source)
                .err()
                .map(|error| error.to_string());
            // Then its original Serde error, including the map expectation, survives.
            assert_eq!(result, Some(message.to_string()));
        }
    }

    #[test]
    fn rejects_non_string_keys_when_deserializing_entries() {
        // Given a map stream with an integer key rather than a filename.
        let deserializer = MapDeserializer::<_, Error>::new([(1_u32, 2_usize)].into_iter());
        // When the typed key seed consumes the first pair.
        let result = FileMap::deserialize(deserializer)
            .err()
            .map(|error| error.to_string());
        // Then key decoding fails rather than coercing or inserting it.
        assert_eq!(
            result,
            Some("invalid type: integer `1`, expected a string".to_string())
        );
    }
}
