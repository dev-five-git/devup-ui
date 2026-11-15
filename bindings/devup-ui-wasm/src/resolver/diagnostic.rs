use super::capabilities::Resolver;

impl<V: Clone, F, S> Resolver<V, F, S> {
    pub(crate) fn diagnostic(&self, value: &V) -> String {
        self.state.with(|state| state.remember(value));
        if let Some(message) = (self.values.as_string)(value) {
            return message;
        }
        if let Ok(message) = (self.get)(value, &(self.values.string)("message"))
            && let Some(message) = (self.values.as_string)(&message)
        {
            return message;
        }
        match (self.json.stringify)(value) {
            Ok(text) => (self.values.as_string)((self.json.value)(&text))
                .unwrap_or_else(|| "undefined".to_string()),
            Err(_) => "<unprintable JavaScript exception>".to_string(),
        }
    }
}
