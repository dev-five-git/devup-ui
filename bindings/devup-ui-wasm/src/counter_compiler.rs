use crate::prospective_output::{OutputMetadata, OutputRoute, ProspectiveSheet};
use crate::{Output, extraction_rollback::ExtractionRollback, with_style_sheet_mut};
use extractor::compiler_policy::{CompilerInput, CounterCompileError, with_counter_extract};
use sheet::counter_kernel::{CounterSheet, UpdateError, UpdateRequest};
use std::convert::Infallible;

pub(super) fn code_extract_counter(
    input: CompilerInput<'_>,
    import_main_css: bool,
) -> Result<Output, String> {
    transaction(input, import_main_css, || Ok::<_, Infallible>(())).map_err(|error| match error {
        CounterCompileError::Compile(error) => error.to_string(),
        CounterCompileError::Consumer(error) => error.to_string(),
    })
}

fn transaction<E>(
    input: CompilerInput<'_>,
    import_main_css: bool,
    after_finish: impl FnOnce() -> Result<(), E>,
) -> Result<Output, CounterCompileError<UpdateError<E>>> {
    css::admission::with_admission(|| {
        let rollback = ExtractionRollback::capture();
        let route = OutputRoute {
            raw_source: input.filename,
            single_css: input.option.single_css,
            import_main_css,
        };
        let result = css::exact_attempt::with_exclusive_attempt(|| {
            with_counter_extract(input, |metadata, batch| {
                with_style_sheet_mut(|sheet| {
                    CounterSheet::new(sheet).with_attempt(|attempt| {
                        let completed = attempt
                            .prepare_compiled(
                                batch,
                                UpdateRequest {
                                    raw_source: route.raw_source,
                                    single_css: route.single_css,
                                },
                            )?
                            .finish(|sheet, effects| {
                                Ok(Output::from_prospective(
                                    OutputMetadata {
                                        code: metadata.code,
                                        map: metadata.map,
                                        css_file: metadata.css_file,
                                        dependencies: metadata.dependencies,
                                    },
                                    ProspectiveSheet { sheet, effects },
                                    route,
                                ))
                            })?;
                        after_finish().map_err(UpdateError::Output)?;
                        Ok(completed)
                    })
                })
            })
        });
        if result.is_ok() {
            rollback.commit();
        }
        result
    })
}

#[cfg(test)]
pub(super) fn after_finish<E>(
    input: CompilerInput<'_>,
    hook: impl FnOnce() -> Result<(), E>,
) -> Result<Output, CounterCompileError<UpdateError<E>>> {
    transaction(input, false, hook)
}
