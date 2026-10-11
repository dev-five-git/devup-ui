use css::file_map::{canonical, is_global};
use extractor::{ExtractGraphOutput, ExtractOutput};

use super::{Output, with_style_sheet_mut};

pub(super) fn publish(graph: ExtractGraphOutput, filename: &str, mode: (bool, bool)) -> Output {
    let (single, import_main) = mode;
    let canonical_entry = canonical(filename);
    let global_entry = single || is_global(filename);
    with_style_sheet_mut(|sheet| {
        let mut collected = false;
        let mut base = false;
        let mut removed = false;
        for (raw, output) in graph
            .artifacts
            .iter()
            .map(|artifact| (artifact.filename.as_str(), &artifact.output))
            .chain(std::iter::once((filename, &graph.entry)))
        {
            let global = single || is_global(raw);
            removed |= sheet.rm_global_css(raw, global);
            let changed = sheet.update_styles(&output.styles, &canonical(raw), global);
            collected |= changed.0;
            base |= changed.1;
        }
        let css = (collected || base || removed).then(|| {
            sheet.create_css(
                if global_entry {
                    None
                } else {
                    Some(&canonical_entry)
                },
                import_main,
            )
        });
        let ExtractOutput {
            code,
            map,
            css_file,
            dependencies,
            styles: _,
        } = graph.entry;
        Output {
            code,
            map,
            css_file,
            dependencies,
            css,
            updated_base_style: base || removed,
        }
    })
}
