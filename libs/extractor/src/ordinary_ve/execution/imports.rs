use super::{SelectedModule, policy};
use crate::module_loader::Mapped;
use crate::ordinary_ve::selection::plan::{ImportName, MemberDemand, NativeBinding};
use crate::vanilla_extract::json_string;

pub(super) fn write(
    module: SelectedModule<'_>,
    option: &crate::ExtractOption,
    mapped: &mut Mapped,
) -> Result<(), String> {
    let SelectedModule {
        stylesheet,
        selection,
    } = module;
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
            Some(NativeBinding::Named { api, .. }) => (
                format!("{{ {api} as {} }}", import.binding.name),
                option.package.as_str(),
            ),
            Some(NativeBinding::Namespace) => (
                format!("* as {}", import.binding.name),
                option.package.as_str(),
            ),
            None => (
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
        if import.native.is_some() && !option.import_aliases.contains_key("@vanilla-extract/css") {
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
