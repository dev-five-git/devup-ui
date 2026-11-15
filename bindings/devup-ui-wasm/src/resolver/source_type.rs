use super::capabilities::Resolver;

impl<V: Clone, F, S> Resolver<V, F, S> {
    pub(crate) fn source_type(
        &self,
        value: &V,
    ) -> Result<Option<extractor::ExtractSourceType>, String> {
        if (self.values.is_undefined)(value) || (self.values.is_null)(value) {
            return extractor::parse_source_type(None);
        }
        let value = (self.values.as_string)(value).ok_or_else(|| {
            "source type cannot use a non-string at build time: expected `compiled-mdx`".to_string()
        })?;
        extractor::parse_source_type(Some(&value))
    }
}
