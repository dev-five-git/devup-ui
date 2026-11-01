//! Converts host-held observations into failures without trusting caught CSS errors.

use boa_engine::{JsError, JsNativeError};

use super::{Failure, Kind, Sandbox, Violation};

impl Sandbox {
    /// Fails if a forbidden read was recorded. `errors` are the errors the host
    /// got back from code it ran: one a read threw gives that read its call
    /// frames when the code did not rethrow it first, so a host that serializes
    /// the values the code made passes the errors that failed it here.
    pub(crate) fn check(&self, errors: &[&JsError]) -> Result<(), Failure> {
        let recorded = self.evidence.reads.borrow();
        if recorded.is_empty() {
            return Ok(());
        }
        Err(Failure::Forbidden(
            recorded
                .iter()
                .map(|read| Violation {
                    kind: if read.css.is_some() {
                        Kind::Css
                    } else {
                        Kind::of(&read.name)
                    },
                    name: read.name.clone(),
                    css_message: read.css.as_ref().map(|css| css.message.clone()),
                    site: read.site.clone(),
                    error: match (&read.css, &read.site) {
                        (Some(css), _) => css.error.clone(),
                        (None, Some((place, _))) => JsNativeError::reference()
                            .with_message(format!(
                                "`{}` cannot be read at build time: {}\n    at <read> ({place})",
                                read.name,
                                Kind::of(&read.name).requirement()
                            ))
                            .into(),
                        (None, None) => errors
                            .iter()
                            .find(|error| {
                                error
                                    .as_opaque()
                                    .is_some_and(|thrown| thrown.strict_equals(&read.error))
                            })
                            .map_or_else(
                                || JsError::from_opaque(read.error.clone()),
                                |error| (*error).clone(),
                            ),
                    },
                })
                .collect(),
        ))
    }
}
