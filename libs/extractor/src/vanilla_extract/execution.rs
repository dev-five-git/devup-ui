//! Both input modes share one loader, sandbox and native allocation transaction.

use super::{
    CollectedStyles, Collector, Context, Evaluating, IMPORTED_RUNS, ModuleLoader, NameScope, Path,
    Rc, RefCell, SCRIPT_PATH, Script, Source, StyleCollector, Stylesheet, StylesheetImports, Unit,
    capture, get_file_num_by_filename, module_script, register_vanilla_extract_apis,
    top_level_bindings,
};

mod compiled;
mod diagnostics;

pub(super) enum Input<'a> {
    Stylesheet(Stylesheet<'a>, bool),
    Selected(capture::Selected<'a>),
    Readback(capture::Selected<'a>, &'a [oxc_span::Span]),
}

impl Input<'_> {
    const fn stylesheet(&self) -> Stylesheet<'_> {
        match self {
            Self::Stylesheet(stylesheet, _) => *stylesheet,
            Self::Selected(selected) | Self::Readback(selected, _) => selected.stylesheet,
        }
    }
}

pub(crate) struct Executed {
    pub compiled: crate::stylesheet_compile::Compiled,
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
    let stylesheet = input.stylesheet();
    let mut loader = ModuleLoader::new(resolver, option);
    loader.select_demands(stylesheet, !matches!(&input, Input::Stylesheet(_, _)))?;
    run(input, loader)
}

pub(super) fn run(input: Input<'_>, mut loader: ModuleLoader<'_>) -> Result<Executed, String> {
    let stylesheet = input.stylesheet();
    let Stylesheet {
        filename,
        code,
        source,
        edits,
    } = stylesheet;
    let _evaluating = Evaluating::enter(filename);
    let unit = match &input {
        Input::Stylesheet(_, _) => Unit::written(filename, code, source, edits)?,
        Input::Selected(selected) | Input::Readback(selected, _) => {
            Unit::selected(stylesheet, selected.mapped)?
        }
    };
    let mut entry = module_script(&unit, &mut loader, true)?;
    let file_num = get_file_num_by_filename(filename);
    let compiled = match &input {
        Input::Stylesheet(_, true) => {
            crate::stylesheet_compile::compile(stylesheet, loader.option(), loader.resolver())?
        }
        Input::Stylesheet(_, false) | Input::Selected(_) | Input::Readback(_, _) => {
            crate::stylesheet_compile::Compiled::default()
        }
    };
    if let Some(replacement) = compiled::entry(stylesheet, &compiled, &mut loader)? {
        entry = replacement;
    }
    let run = loader.script(&entry);
    let mut imports = StylesheetImports {
        artifacts: std::mem::take(&mut loader.artifacts),
        dependencies: std::mem::take(&mut loader.dependencies),
        kept_imports: std::mem::take(&mut loader.kept_imports),
        atoms: loader.imported_atoms.clone(),
        references: loader.imported_references.clone(),
    };
    let (bucket, global, _) = crate::resolve_css_target(filename, loader.option());
    compiled::seed(
        &compiled,
        &mut imports,
        if global { None } else { Some(bucket.as_str()) },
    );
    let imported =
        crate::module_loader::evaluating_import() && matches!(&input, Input::Stylesheet(_, _));
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
            compiled,
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
    if let Some(evaluation) = &compiled.evaluation {
        crate::evaluation_sandbox::defer_rendering(&mut context, evaluation)
            .map_err(|error| run.explain(&error.to_string(), filename))?;
    }
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
        Input::Stylesheet(_, _) => {}
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
    let diagnostics = diagnostics::Diagnostics {
        run: &run,
        operations: &operations,
        instrumented: &instrumented,
        filename,
        required: loader.has_demands(),
    };
    sandbox
        .run_source(
            &mut context,
            Source::from_bytes(instrumented.code.as_bytes()).with_path(Path::new(SCRIPT_PATH)),
        )
        .map_err(|failure| diagnostics.explain(failure, &context))?;
    if loader.has_demands() {
        super::demand_runtime::validate(&context)?;
    }
    let mut collected = std::mem::take(&mut collector.borrow_mut().styles);
    let mut exports = Vec::new();
    let named = match &input {
        Input::Stylesheet(_, _) => {
            let reserved = super::naming::reserved(stylesheet);
            top_level_bindings(
                compiled
                    .evaluation
                    .as_ref()
                    .map_or(code, |evaluation| evaluation.code.as_str()),
                &entry,
                &mut context,
            )
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
        .map_err(|failure| diagnostics.explain(failure, &context))?;
    let finished = named.map_err(|error| match error {
        capture::FinalizeError::Located(message) => message,
        capture::FinalizeError::Js(error) => run.explain(
            &operations.explain(&instrumented.explain(&error.to_string()), &context),
            filename,
        ),
    })?;
    compiled::exports(&compiled, &mut exports);
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
        compiled,
        collected,
        imports,
        captures: finished.captures,
        emission: finished.emission,
        readback: finished.readback,
        exports,
    })
}
