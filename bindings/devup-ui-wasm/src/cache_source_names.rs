use css::content_name::ContentName;
use sheet::name_registry::NameRegistry;

pub(crate) fn validate(text: &str, names: &NameRegistry) -> Result<(), String> {
    let mut scanned = 0;
    for (start, _) in text.match_indices("---") {
        if start < scanned {
            continue;
        }
        let tail = &text[start..];
        let end = tail
            .find(|character: char| {
                !(character.is_ascii_alphanumeric()
                    || character >= '\u{80}'
                    || matches!(character, '-' | '_'))
            })
            .unwrap_or(tail.len());
        scanned = start + end;
        let variable = &tail[..end];
        let Some(marker) = variable.rfind(|character: char| character.is_ascii_uppercase()) else {
            continue;
        };
        let (prefix, file_part) = match variable.as_bytes()[marker] {
            b'L' | b'H' if variable[..marker].ends_with("SU") => {
                (&variable[..marker - 1], &variable[marker - 1..])
            }
            b'U' if variable[..marker].ends_with('S') => {
                return Err(format!(
                    "cached source variable `{variable}` uses an unprotected source naming contract; rebuild all caches"
                ));
            }
            _ => continue,
        };
        let Some((file_part, _position)) = file_part.split_once('-') else {
            return Err(format!(
                "cached source variable `{variable}` has no source position; rebuild all caches"
            ));
        };
        let name = &variable[..prefix.len() + file_part.len()];
        let Some(claim) = names.get(name) else {
            return Err(format!(
                "cached source variable `{variable}` has no exact source claim; rebuild all caches with the current naming contract"
            ));
        };
        if claim.descriptor != claim.content.as_bytes()
            || ContentName::source(&claim.content).name(prefix) != name
        {
            return Err(format!(
                "cached source variable `{variable}` has an invalid exact source claim; rebuild all caches with the current naming contract"
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use css::style_origin::RealLocation;
    use rstest::rstest;
    use sheet::name_registry::NameClaim;

    #[rstest]
    #[case("---Sa-a", true)]
    #[case("---Mixed-SUH-prefix-Sa-b-c", true)]
    #[case("--theme-color", true)]
    #[case("--v1-color-0-selector", true)]
    #[case("plain text", true)]
    #[case("---lowercase", true)]
    #[case("var(---SULa-b)", true)]
    #[case("---SULa-b-c", true)]
    #[case("var(---SULmissing-b)", false)]
    #[case("---SULa", false)]
    #[case("var(---SUa-b)", false)]
    fn cached_source_variables_require_exact_claims_without_confusing_other_namespaces(
        #[case] text: &str,
        #[case] protected: bool,
    ) {
        // Given: one exact short source claim and arbitrary cached CSS tokens.
        let names = NameRegistry::from([(
            "---SULa".into(),
            NameClaim {
                descriptor: b"a".to_vec(),
                content: "a".into(),
                origin: None,
                location: RealLocation::ModuleExport {
                    file: "/real/a.tsx".into(),
                    binding: None,
                },
            },
        )]);
        // When: the production cache boundary scans both reset/value token shapes.
        let result = validate(text, &names);
        // Then: only source-variable tokens require source proof and a position.
        assert_eq!(result.is_ok(), protected, "{result:?}");
    }
}
