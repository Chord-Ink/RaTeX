use super::EnvSpec;
use crate::editor::{Argument, ArgumentKind, ArgumentShape, CatalogEntry, EntryKind, Syntax};

pub(crate) fn entry(name: &'static str, spec: &EnvSpec) -> CatalogEntry {
    let mut entry = CatalogEntry::new(name, EntryKind::Environment);
    entry.syntax = Syntax::Environment;
    let base = name.strip_suffix('*').unwrap_or(name);
    let (arguments, body) = match base {
        "array" | "darray" | "subarray" => (
            vec![Argument::new(
                ArgumentKind::Columns,
                ArgumentShape::Required,
                "c",
            )],
            "x",
        ),
        "alignat" | "alignedat" => (
            vec![Argument::new(
                ArgumentKind::Count,
                ArgumentShape::Required,
                "1",
            )],
            "x&=y",
        ),
        "matrix" | "pmatrix" | "bmatrix" | "Bmatrix" | "vmatrix" | "Vmatrix" if entry.starred => (
            vec![Argument::new(
                ArgumentKind::Alignment,
                ArgumentShape::Optional,
                "c",
            )],
            "x",
        ),
        "prooftree" => (Vec::new(), r"\AxiomC{A}"),
        _ => (Vec::new(), "x"),
    };
    debug_assert_eq!(
        arguments
            .iter()
            .filter(|a| a.shape == ArgumentShape::Required)
            .count(),
        spec.num_args
    );
    entry.arguments = arguments;
    let mut sample = format!(r"\begin{{{name}}}");
    for argument in &entry.arguments {
        let (open, close) = if argument.shape == ArgumentShape::Optional {
            ('[', ']')
        } else {
            ('{', '}')
        };
        sample.push(open);
        sample.push_str(argument.sample);
        sample.push(close);
    }
    sample.push_str(body);
    sample.push_str(&format!(r"\end{{{name}}}"));
    entry.sample = Some(sample);
    entry
}
