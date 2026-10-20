//! Move-based isolation for the globals scratch extraction can mutate.

use crate::{module_loader::EvaluationGuard, vanilla_extract::CacheGuard};

/// Restores warmed stylesheet caches and any outer evaluation stack on drop.
#[derive(Default, Debug)]
pub struct ScratchExtractionState {
    _caches: CacheGuard,
    _evaluation: EvaluationGuard,
}

impl ScratchExtractionState {
    /// Discard cached names that may refer to slots removed with a failed row.
    pub fn discard_failed_row(&self) {
        crate::vanilla_extract::discard_reserved_references();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExtractOption, ModuleResolution, ResolvedModule, extract_with_modules};
    use serial_test::serial;

    fn supplier(specifier: &str, _: &str) -> Option<ModuleResolution> {
        (specifier == "./supplier.css").then(|| {
            ModuleResolution::Resolved(ResolvedModule {
            path: "supplier.css.ts".into(),
            code:
                "import { style } from '@devup-ui/react'; export const base = style({color:'red'});"
                    .into(),
            source_type: None,
        })
        })
    }

    #[test]
    #[serial]
    fn warmed_caches_restore_when_scratch_succeeds_or_unwinds()
    -> Result<(), Box<dyn std::error::Error>> {
        // Given
        let _isolated = ScratchExtractionState::default();
        let source = "import {style} from '@devup-ui/react'; import {base} from './supplier.css'; export const own = style([base,{width:'12px'}]);";
        extract_with_modules(
            "root.css.ts",
            source,
            ExtractOption::default(),
            false,
            &supplier,
        )?;
        let mode_source = "export const view=<div/>; export const token='red';";
        for mode in [None, Some(crate::ExtractSourceType::CompiledMdx)] {
            crate::vanilla_extract::strip_typescript_with_type(mode_source, "mode-cache.ts", mode)?;
        }
        let outer = crate::module_loader::Evaluating::enter("outer.css.ts");
        let saved = ScratchExtractionState::default();
        let before = format!("{saved:?}");
        assert!(before.contains("supplier.css.ts"));
        drop(saved);
        // When
        for unwind in [false, true] {
            let result = std::panic::catch_unwind(|| {
                let _scratch = ScratchExtractionState::default();
                for mode in [None, Some(crate::ExtractSourceType::CompiledMdx)] {
                    crate::vanilla_extract::strip_typescript_with_type(
                        "export const token='blue';",
                        "mode-cache.ts",
                        mode,
                    )
                    .unwrap_or_else(|error| panic!("{error}"));
                }
                assert!(!crate::module_loader::loading_for_stylesheet());
                let _inner = crate::module_loader::Evaluating::enter("scratch.css.ts");
                assert!(crate::module_loader::loading_for_stylesheet());
                assert!(
                    extract_with_modules(
                        "scratchroot.css.ts",
                        source,
                        ExtractOption::default(),
                        false,
                        &supplier
                    )
                    .is_ok()
                );
                assert!(!unwind, "scratch unwind");
            });
            assert_eq!(result.is_err(), unwind);
            // Then
            let after = ScratchExtractionState::default();
            assert_eq!(format!("{after:?}"), before);
            drop(after);
            assert!(crate::module_loader::loading_for_stylesheet());
        }
        drop(outer);
        Ok(())
    }
}
