use bimap::BiHashMap;
use serde::de::{DeserializeOwned, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};
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

pub(crate) fn decode<T: DeserializeOwned>(value: JsValue, slot: u32) -> Result<T, JsValue> {
    begin(slot);
    match serde_wasm_bindgen::from_value(value) {
        Ok(value) => {
            end();
            Ok(value)
        }
        Err(cause) => Err(error(JsValue::from(cause))),
    }
}

#[cfg(test)]
mod tests {
    use super::FileMap;
    use serde::Deserialize;
    use serde::de::value::{Error, MapDeserializer};

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
