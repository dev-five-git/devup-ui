//! Both input modes share one loader, sandbox and native allocation transaction.

use super::{
    CollectedStyles, Collector, Context, Evaluating, IMPORTED_RUNS, ModuleLoader, NameScope, Path,
    Rc, RefCell, SCRIPT_PATH, Script, Source, StyleCollector, Stylesheet, StylesheetImports, Unit,
    capture, get_file_num_by_filename, module_script, register_vanilla_extract_apis, sandbox_error,
    top_level_bindings,
};

pub(super) enum Input<'a> {
    Stylesheet(Stylesheet<'a>),
    Selected(capture::Selected<'a>),
    Readback(capture::Selected<'a>, &'a [oxc_span::Span]),
}

pub(crate) struct Executed {
    pub collected: CollectedStyles,
    pub imports: StylesheetImports,
    pub captures: Vec<String>,
    pub emission: capture::Emission,
    pub readback: Vec<(oxc_span::Span, String)>,
    pub(super) exports: Vec<super::naming::ExportValue>,
}

pub(super) struct ImportedRun {
    pub file_num: usize,
    pub code: String,
    pub script: String,
    pub collected: CollectedStyles,
    pub exports: Vec<super::naming::ExportValue>,
}

pub(super) fn execute(
    input: Input<'_>,
    option: &crate::ExtractOption,
    resolver: Option<&crate::ModuleResolver>,
) -> Result<Executed, String> {
    let stylesheet = match &input {
        Input::Stylesheet(stylesheet) => *stylesheet,
        Input::Selected(selected) | Input::Readback(selected, _) => selected.stylesheet,
    };
    let mut loader = ModuleLoader::new(resolver, option);
    loader.select_demands(stylesheet, !matches!(&input, Input::Stylesheet(_)))?;
    run(input, loader)
}

pub(super) fn run(input: Input<'_>, mut loader: ModuleLoader<'_>) -> Result<Executed, String> {
    let stylesheet = match &input {
        Input::Stylesheet(stylesheet) => *stylesheet,
        Input::Selected(selected) | Input::Readback(selected, _) => selected.stylesheet,
    };
    let Stylesheet {
        filename,
        code,
        source,
        edits,
    } = stylesheet;
    let _evaluating = Evaluating::enter(filename);
    let unit = match &input {
        Input::Stylesheet(_) => Unit::written(filename, code, source, edits)?,
        Input::Selected(selected) | Input::Readback(selected, _) => {
            Unit::selected(stylesheet, selected.mapped)?
        }
    };
    let entry = module_script(&unit, &mut loader, true)?;
    let file_num = get_file_num_by_filename(filename);
    let run = loader.script(&entry);
    let mut imports = StylesheetImports {
        dependencies: std::mem::take(&mut loader.dependencies),
        kept_imports: std::mem::take(&mut loader.kept_imports),
        atoms: loader.imported_atoms.clone(),
        references: loader.imported_references.clone(),
    };
    let imported =
        crate::module_loader::evaluating_import() && matches!(&input, Input::Stylesheet(_));
    if imported
        && loader.producers().is_empty()
        && let Some((collected, exports)) = IMPORTED_RUNS.with_borrow(|runs| {
            runs.get(filename)
                .filter(|cached| {
                    cached.file_num == file_num && cached.code == code && cached.script == run.text
                })
                .map(|cached| (cached.collected.clone(), cached.exports.clone()))
        })
    {
        return Ok(Executed {
            collected,
            imports,
            captures: Vec::new(),
            emission: capture::Emission::default(),
            readback: Vec::new(),
            exports,
        });
    }
    let collector: StyleCollector = Rc::new(RefCell::new(Collector {
        file_num,
        imported_atoms: Rc::new(imports.atoms.clone()),
        imported_references: Rc::new(imports.references.clone()),
        ..Collector::default()
    }));
    let mut context = Context::default();
    let sandbox = crate::evaluation_sandbox::Sandbox::new(&mut context)
        .map_err(|error| run.explain(&error.to_string(), filename))?;
    loader
        .prepare_css(&mut context)
        .map_err(|error| run.explain(&error.to_string(), filename))?;
    let operations = crate::module_loader::operations::Operations::new(&run.text);
    operations
        .prepare(&mut context)
        .map_err(|error| run.explain(&error.to_string(), filename))?;
    match &input {
        Input::Selected(selected) | Input::Readback(selected, _) => capture::prepare(
            &mut context,
            selected,
            capture::Samplers {
                reads: entry.reads.clone(),
                namespaces: loader.namespaces(),
            },
        )
        .map_err(|error| run.explain(&error.to_string(), filename))?,
        Input::Stylesheet(_) => {}
    }
    let instrumented = crate::evaluation_sandbox::instrument(&operations.code, SCRIPT_PATH);
    sandbox
        .prepare(&mut context, &instrumented)
        .map_err(|error| run.explain(&instrumented.explain(&error.to_string()), filename))?;
    if loader.has_demands() {
        super::demand_runtime::prepare(&mut context, &loader, &collector)
            .map_err(|error| Script::default().explain(&error, filename))?;
    } else {
        register_vanilla_extract_apis(&mut context, &collector)
            .map_err(|error| Script::default().explain(&error, filename))?;
    }
    let explain = |failure: crate::evaluation_sandbox::Failure, context: &Context| {
        let required = loader
            .has_demands()
            .then(|| {
                crate::ordinary_ve::execution::diagnostics::required(&failure, &run, &operations)
            })
            .flatten();
        let message = sandbox_error(
            failure,
            &run,
            |error| operations.explain(&instrumented.explain(error), context),
            filename,
        );
        match required {
            Some(required) => format!("{message}\n{required}"),
            None => message,
        }
    };
    sandbox
        .run_source(
            &mut context,
            Source::from_bytes(instrumented.code.as_bytes()).with_path(Path::new(SCRIPT_PATH)),
        )
        .map_err(|failure| explain(failure, &context))?;
    if loader.has_demands() {
        super::demand_runtime::validate(&context)?;
    }
    let mut collected = std::mem::take(&mut collector.borrow_mut().styles);
    let mut exports = Vec::new();
    let named = match &input {
        Input::Stylesheet(_) => {
            let reserved = super::naming::reserved(stylesheet);
            top_level_bindings(code, &entry, &mut context)
                .and_then(|bindings| {
                    NameScope {
                        file_num,
                        reserved: &reserved,
                    }
                    .name_entries(&mut collected, &bindings, &mut context)
                })
                .map(|values| {
                    exports = values;
                    capture::Finished::default()
                })
                .map_err(capture::FinalizeError::Js)
        }
        Input::Selected(selected) => capture::finish((selected, &[]), &mut collected, &mut context),
        Input::Readback(selected, reads) => {
            capture::finish((selected, reads), &mut collected, &mut context)
        }
    };
    sandbox
        .check(
            &named
                .as_ref()
                .err()
                .and_then(|error| match error {
                    capture::FinalizeError::Js(error) => Some(error),
                    capture::FinalizeError::Located(_) => None,
                })
                .into_iter()
                .collect::<Vec<_>>(),
        )
        .map_err(|failure| explain(failure, &context))?;
    let finished = named.map_err(|error| match error {
        capture::FinalizeError::Located(message) => message,
        capture::FinalizeError::Js(error) => run.explain(
            &operations.explain(&instrumented.explain(&error.to_string()), &context),
            filename,
        ),
    })?;
    if loader.has_demands() {
        super::demand_runtime::finish(&mut context, &mut imports)?;
    }
    if imported && loader.producers().is_empty() {
        IMPORTED_RUNS.with_borrow_mut(|runs| {
            runs.insert(
                filename.to_string(),
                ImportedRun {
                    file_num,
                    code: code.to_string(),
                    script: run.text,
                    collected: collected.clone(),
                    exports: exports.clone(),
                },
            );
        });
    }
    Ok(Executed {
        collected,
        imports,
        captures: finished.captures,
        emission: finished.emission,
        readback: finished.readback,
        exports,
    })
}
