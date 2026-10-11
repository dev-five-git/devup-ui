use super::{SelectedModule, policy};
use crate::module_loader::Mapped;
use crate::ordinary_ve::selection::plan::{ImportName, MemberDemand, NativeBinding};
use crate::vanilla_extract::json_string;

pub(crate) fn native_name(
    selection: &crate::ordinary_ve::selection::Selection,
    api: &str,
) -> String {
    let mut name = format!("__ve_api_{api}__");
    while selection.reserved_names.contains(&name) {
        name.push('$');
    }
    name
}

fn native_reader(selection: &crate::ordinary_ve::selection::Selection) -> String {
    let mut name = String::from("__ve_native_read__");
    while selection.reserved_names.contains(&name) {
        name.push('$');
    }
    name
}

pub(crate) fn native_read(
    source: &str,
    selection: &crate::ordinary_ve::selection::Selection,
    call: &crate::ordinary_ve::selection::plan::NativeCall,
) -> String {
    // An identifier-led callee preserves ASI without inserting a statement that
    // could steal an unbraced branch/loop body. Read the original before the API.
    format!(
        "{}(({}), {})",
        native_reader(selection),
        call.callee.source_text(source),
        native_name(selection, call.api)
    )
}

pub(crate) fn copy(module: SelectedModule<'_>, span: oxc_span::Span, mapped: &mut Mapped) {
    let source = module.stylesheet.code;
    let selection = module.selection;
    let mut start = span.start;
    let mut edits: Vec<_> = selection
        .checks
        .iter()
        .filter(|check| span.contains_inclusive(check.span))
        .map(|check| {
            let message = format!(
                "{}: {}. Fix: {}",
                policy::place(module.stylesheet, check.span.start),
                check.cause(),
                check.fix()
            );
            (
                check.span,
                format!("(function(){{throw {};}})()", json_string(&message)),
            )
        })
        .collect();
    for call in &selection.native_calls {
        if span.contains_inclusive(call.callee)
            && !edits
                .iter()
                .any(|(check, _)| call.callee.contains_inclusive(*check))
        {
            edits.push((call.callee, native_read(source, selection, call)));
        }
    }
    edits.sort_by_key(|(span, _)| span.start);
    for (edit, text) in edits {
        if edit.start < start {
            continue;
        }
        mapped.copy(source, oxc_span::Span::new(start, edit.start));
        mapped.synthesize(edit.start, &text);
        start = edit.end;
    }
    mapped.copy(source, oxc_span::Span::new(start, span.end));
}

pub(in crate::ordinary_ve) fn write(
    module: SelectedModule<'_>,
    option: &crate::ExtractOption,
    mapped: &mut Mapped,
) -> Result<(), String> {
    let SelectedModule {
        stylesheet,
        selection,
    } = module;
    if let Some(call) = selection.native_calls.first() {
        mapped.synthesize(
            call.callee.start,
            &format!(
                "function {}(_original, owner){{return owner;}}\n",
                native_reader(selection)
            ),
        );
    }
    let mut generated = std::collections::BTreeSet::new();
    for call in &selection.native_calls {
        if generated.insert(call.api) {
            mapped.synthesize(
                call.callee.start,
                &format!(
                    "import {{ {} as {} }} from {};\n",
                    call.api,
                    native_name(selection, call.api),
                    json_string(&option.package)
                ),
            );
        }
    }
    for import in &selection.imports {
        if import.erased {
            let (site, input) = selection
                .demands
                .iter()
                .find(|demand| demand.symbol == import.binding.symbol)
                .map_or_else(
                    || (import.specifier, import.binding.name.clone()),
                    |demand| {
                        let input = match &demand.member {
                            MemberDemand::Path(path) => {
                                format!("{}.{}", import.binding.name, path.join("."))
                            }
                            MemberDemand::Whole => import.binding.name.clone(),
                        };
                        (demand.read, input)
                    },
                );
            return Err(format!(
                "{}: required native input `{input}` comes from a type-only import. Fix: import a runtime value",
                policy::place(stylesheet, site.start)
            ));
        }
        let (specifier, package) = match import.native {
            Some(NativeBinding::Named { api, .. })
                if import.source == "@vanilla-extract/css" || import.source == option.package =>
            {
                (
                    format!("{{ {api} as {} }}", import.binding.name),
                    option.package.as_str(),
                )
            }
            Some(NativeBinding::Namespace)
                if import.source == "@vanilla-extract/css" || import.source == option.package =>
            {
                (
                    format!("* as {}", import.binding.name),
                    option.package.as_str(),
                )
            }
            Some(NativeBinding::Named { .. } | NativeBinding::Namespace) | None => (
                match &import.imported {
                    ImportName::Named(name) => {
                        format!("{{ {} as {} }}", json_string(name), import.binding.name)
                    }
                    ImportName::Default => import.binding.name.clone(),
                    ImportName::Namespace => format!("* as {}", import.binding.name),
                },
                import.source.as_str(),
            ),
        };
        if import.native.is_some()
            && import.source != option.package
            && !option.import_aliases.contains_key("@vanilla-extract/css")
        {
            return Err(format!(
                "{}: native slice execution requires the enabled vanilla-extract alias",
                policy::place(stylesheet, import.specifier.start)
            ));
        }
        mapped.synthesize(
            import.specifier.start,
            &format!("import {specifier} from {};\n", json_string(package)),
        );
    }
    Ok(())
}
