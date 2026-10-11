use super::*;
use css::style_selector::StyleSelector;
use extractor::extract_style::ExtractStyleProperty;

pub(super) type TestResult = Result<(), String>;

pub(super) fn setup() {
    import_sheet_internal(StyleSheet::default());
    css::class_map::reset_class_map();
    css::file_map::reset_file_map();
    css::file_map::reset_canonical_map();
    css::file_routes::reset_file_routes();
    set_atom_hoist(None);
    set_prefix(None);
}

pub(super) fn aliases() -> HashMap<String, ImportAlias> {
    HashMap::from([("@vanilla-extract/css".into(), ImportAlias::NamedToNamed)])
}

pub(super) struct Graph {
    pub entry: String,
    pub origin: String,
    pub source: String,
    pub modules: HashMap<String, ResolvedModule>,
    pub single: bool,
}

impl Graph {
    pub fn resolver(&self) -> Box<ModuleResolver> {
        let modules: HashMap<_, _> = self
            .modules
            .iter()
            .map(|(specifier, module)| {
                (
                    specifier.clone(),
                    (module.path.clone(), module.code.clone()),
                )
            })
            .collect();
        Box::new(move |specifier: &str, _: &str| {
            let (path, code) = modules.get(specifier)?;
            Some(ResolvedModule {
                path: path.clone(),
                code: code.clone(),
            })
        })
    }

    pub fn extract(&self) -> Result<Output, String> {
        let resolver = self.resolver();
        code_extract_with_modules_internal(
            &self.entry,
            &self.source,
            "@devup-ui/react",
            "df".into(),
            self.single,
            false,
            false,
            aliases(),
            resolver.as_ref(),
        )
    }

    pub fn replace_origin(&mut self, source: &str) {
        self.modules.insert(
            "./origin".into(),
            ResolvedModule {
                path: self.origin.clone(),
                code: source.into(),
            },
        );
    }
}

pub(super) fn roots(values: (&str, &str, &str)) -> String {
    let (space, color, opacity) = values;
    format!(
        concat!(
            "import * as ve from '@vanilla-extract/css';let runs=0;",
            "(runs++,ve.createVar());export const [theme,vars]=(runs++,ve.createTheme({{space:'{}'}}));",
            "export const base=(runs++,ve.style({{color:'red',padding:8}}));",
            "export const spin=(runs++,ve.keyframes({{to:{{opacity:{}}}}}));",
            "(runs++,ve.globalStyle('body',{{color:'{}'}}));export function count(){{return runs;}}"
        ),
        space, opacity, color
    )
}

pub(super) fn native(suffix: &str, single: bool) -> Graph {
    let origin = format!("/b-origin.{suffix}");
    let mut graph = Graph {
        entry: format!("/b-entry.{suffix}"), origin,
        source: concat!(
            "import {globalStyle} from '@vanilla-extract/css';import {make,base,count} from './left';",
            "import {vars,spin} from './right';export const box=make([base,{color:'blue',margin:vars.space,animationName:spin,zIndex:count()}]);",
            "globalStyle(`${base}:focus`,{outlineColor:'purple'});const browser=window.document;"
        ).into(),
        modules: HashMap::from([
            ("./left".into(), ResolvedModule { path: format!("/b-left.{suffix}"), code: "export {make,base,count} from './origin';".into() }),
            ("./right".into(), ResolvedModule { path: format!("/b-right.{suffix}"), code: "export {vars,spin} from './origin';".into() }),
        ]), single,
    };
    graph.replace_origin(&format!(
        "{}const key='style';const make=ve[key];export {{make}};const browser=window.document;",
        roots(("8px", "purple", "1"))
    ));
    graph
}

pub(super) struct Names {
    pub theme: String,
    pub variable: String,
    pub spin: String,
    pub focus: String,
}

pub(super) fn names(graph: &Graph) -> Result<Names, String> {
    names_for(graph, ("8px", "purple", "1"))
}

pub(super) fn names_for(graph: &Graph, values: (&str, &str, &str)) -> Result<Names, String> {
    let control = format!(
        "{}ve.globalStyle(`${{base}}:focus`,{{outlineColor:'purple'}});",
        roots(values)
    );
    let output = extract_with_modules(
        &graph.origin,
        &control,
        ExtractOption {
            single_css: graph.single,
            import_aliases: aliases(),
            ..ExtractOption::default()
        },
        false,
        &|_, _| None,
    )
    .map_err(|error| error.to_string())?;
    let (theme, variable) = output
        .styles
        .iter()
        .find_map(|value| match value {
            ExtractStyleValue::Static(style) if style.property.starts_with("--space-") => {
                match &style.selector {
                    Some(StyleSelector::Global(selector, owner)) if owner == &graph.origin => {
                        Some((selector.clone(), style.property.clone()))
                    }
                    _ => None,
                }
            }
            _ => None,
        })
        .ok_or("original-owner theme identity missing")?;
    let spin = output
        .styles
        .iter()
        .find_map(|value| match value {
            ExtractStyleValue::Keyframes(frames) => Some(
                frames
                    .extract(if graph.single {
                        None
                    } else {
                        Some(&graph.origin)
                    })
                    .to_string(),
            ),
            _ => None,
        })
        .ok_or("original-owner keyframes identity missing")?;
    let focus = output
        .styles
        .iter()
        .find_map(|value| match value {
            ExtractStyleValue::Static(style) if style.property == "outline-color" => {
                match &style.selector {
                    Some(StyleSelector::Global(selector, owner)) if owner == &graph.origin => {
                        Some(selector.clone())
                    }
                    _ => None,
                }
            }
            _ => None,
        })
        .ok_or("original-owner selector missing")?;
    Ok(Names {
        theme,
        variable,
        spin,
        focus,
    })
}

pub(super) fn bucket(path: Option<&str>) -> String {
    let css = with_style_sheet(|sheet| sheet.create_css(path, false));
    #[cfg(not(tarpaulin_include))]
    {
        let number = path.map(css::file_map::get_file_num_by_filename);
        assert_eq!(get_css(number, false).ok(), Some(css.clone()));
    }
    css
}

pub(super) fn effects(graph: &Graph, names: &Names, values: (&str, &str, &str)) {
    let (space, color, opacity) = values;
    let global = bucket(None);
    let owner = bucket(if graph.single {
        None
    } else {
        Some(&graph.origin)
    });
    let assignment = format!("{}{{{}:{space}}}", names.theme, names.variable);
    assert_eq!(global.matches(&assignment).count(), 1, "{global}");
    assert_eq!(
        global.matches(&format!("body{{color:{color}}}")).count(),
        1,
        "{global}"
    );
    assert_eq!(
        owner
            .matches(&format!(
                "@keyframes {}{{to{{opacity:{opacity}}}}}",
                names.spin
            ))
            .count(),
        1,
        "{owner}"
    );
    with_style_sheet(|sheet| {
        let key = if graph.single { "" } else { &graph.origin };
        assert!(
            sheet
                .keyframes
                .get(key)
                .is_some_and(|frames| frames.contains_key(&names.spin))
        );
        assert!(sheet.global_css_files.contains(&graph.origin));
    });
}
