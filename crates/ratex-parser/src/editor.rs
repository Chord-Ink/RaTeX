//! Source tokens and the built-in TeX catalog for editors.
//!
//! ```
//! use ratex_parser::editor::{SourceTokens, find};
//!
//! let source = r"\frac  {α}{2}% note";
//! let text: String = SourceTokens::new(source).map(|token| token.text).collect();
//! assert_eq!(text, source);
//! let fraction = find(r"\frac").unwrap();
//! assert_eq!(fraction.arguments.len(), 2);
//! ratex_parser::parse(fraction.sample.as_ref().unwrap()).unwrap();
//! ```

use std::collections::BTreeMap;
use std::sync::LazyLock;

use crate::environments::ENVIRONMENTS;
use crate::functions::{ArgType, FUNCTIONS};
use crate::macro_expander::editor::builtin_macros;
use ratex_font::symbols::{symbols, Mode, SymbolInfo};

pub use ratex_lexer::source::{SourceToken, SourceTokenKind, SourceTokens};
pub use ratex_lexer::token::SourceLocation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryKind {
    Function,
    Environment,
    Symbol,
    Macro,
    Control,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    Public,
    Internal,
}

/// Macro expansion can depend on its arguments and the surrounding mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modes {
    Math,
    Text,
    Both,
    Contextual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgumentShape {
    /// A braced argument, or a single atom where the parser accepts one.
    Required,
    /// An optional argument in square brackets.
    Optional,
    /// An optional literal `*` before the next argument.
    OptionalStar,
    /// A token or group; delimiter and macro-name arguments require a token.
    Token,
    /// A body enclosed by its chosen delimiter, as in `\verb|text|`.
    Delimited,
    /// Input consumed by a special form, rather than an ordinary argument.
    Rest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgumentKind {
    Value(ArgType),
    Delimiter,
    Columns,
    Alignment,
    Count,
    MacroName,
    Parameters,
    EnvironmentName,
    Number,
}

/// Arguments occur in source order. `sample` is the content, without brackets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Argument {
    pub kind: ArgumentKind,
    pub shape: ArgumentShape,
    pub sample: &'static str,
}

impl Argument {
    pub(crate) fn new(kind: ArgumentKind, shape: ArgumentShape, sample: &'static str) -> Self {
        Self {
            kind,
            shape,
            sample,
        }
    }

    pub(crate) fn value(kind: ArgType, shape: ArgumentShape) -> Self {
        let sample = match kind {
            ArgType::Color => "blue",
            ArgType::Size => "1em",
            ArgType::Url => "https://example.invalid/",
            ArgType::Text | ArgType::HBox => "text",
            _ => "x",
        };
        Self::new(ArgumentKind::Value(kind), shape, sample)
    }
}

/// Only `Arguments` has the ordinary optional-then-required brace grammar.
/// Other forms need their named syntax; a sample is a preview, not an insertion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syntax {
    Arguments,
    Infix,
    Delimiter,
    Definition,
    Prefix,
    Switch,
    Environment,
    Verbatim,
    Contextual,
}

#[derive(Debug, Clone)]
pub struct CatalogEntry {
    pub name: &'static str,
    pub kind: EntryKind,
    pub visibility: Visibility,
    pub modes: Modes,
    pub syntax: Syntax,
    pub arguments: Vec<Argument>,
    pub primitive: bool,
    pub infix: bool,
    pub starred: bool,
    /// A bounded, self-contained math input, with text-only entries wrapped.
    /// `None` means the entry needs context for which no sample is supplied.
    pub sample: Option<String>,
    /// All registered modes for this name, even if a macro or function wins.
    pub symbols: Vec<SymbolInfo>,
}

impl CatalogEntry {
    pub(crate) fn new(name: &'static str, kind: EntryKind) -> Self {
        let visibility = if name.contains('@')
            || (name.starts_with(r"\\") && name != r"\\")
            || matches!(name, r"\DOTSI" | r"\DOTSB" | r"\DOTSX")
        {
            Visibility::Internal
        } else {
            Visibility::Public
        };
        Self {
            name,
            kind,
            visibility,
            modes: Modes::Math,
            syntax: Syntax::Arguments,
            arguments: Vec::new(),
            primitive: false,
            infix: false,
            starred: name.ends_with('*'),
            sample: None,
            symbols: Vec::new(),
        }
    }

    pub(crate) fn argument_sample(&self) -> String {
        let mut sample = self.name.to_owned();
        for argument in &self.arguments {
            match argument.shape {
                ArgumentShape::Required | ArgumentShape::Token => {
                    sample.push('{');
                    sample.push_str(argument.sample);
                    sample.push('}');
                }
                ArgumentShape::Optional | ArgumentShape::OptionalStar => {}
                ArgumentShape::Rest | ArgumentShape::Delimited => {
                    sample.push(' ');
                    sample.push_str(argument.sample);
                }
            }
        }
        sample
    }
}

static CATALOG: LazyLock<Vec<CatalogEntry>> = LazyLock::new(build_catalog);

/// Public entries, sorted by name with one entry for each name.
/// Storage is built once and borrowed on later calls.
pub fn catalog() -> impl Iterator<Item = &'static CatalogEntry> + Clone {
    catalog_all()
        .iter()
        .filter(|entry| entry.visibility == Visibility::Public)
}

/// The complete sorted catalog, including internal helpers.
pub fn catalog_all() -> &'static [CatalogEntry] {
    &CATALOG
}

/// Lookup includes internal helpers. Absence says nothing about user macros.
pub fn find(name: &str) -> Option<&'static CatalogEntry> {
    let entries = catalog_all();
    entries
        .binary_search_by_key(&name, |entry| entry.name)
        .ok()
        .map(|index| &entries[index])
}

fn build_catalog() -> Vec<CatalogEntry> {
    let mut entries = BTreeMap::<&str, CatalogEntry>::new();
    for symbol in symbols() {
        let entry = entries.entry(symbol.name).or_insert_with(|| {
            let mut entry = CatalogEntry::new(symbol.name, EntryKind::Symbol);
            entry.modes = match symbol.mode {
                Mode::Math => Modes::Math,
                Mode::Text => Modes::Text,
            };
            entry.sample = Some(format!("{{{}}}", symbol.name));
            entry
        });
        if !entry.symbols.iter().any(|s| s.mode == symbol.mode) {
            entry.symbols.push(symbol);
        }
        if entry.symbols.len() > 1 {
            entry.modes = Modes::Both;
        }
    }
    for (&name, spec) in FUNCTIONS.iter() {
        replace(&mut entries, crate::functions::editor::entry(name, spec));
    }
    for builtin in builtin_macros() {
        replace(&mut entries, builtin.clone());
    }
    for (&name, spec) in ENVIRONMENTS.iter() {
        entries.insert(name, crate::environments::editor::entry(name, spec));
    }
    for name in crate::macro_expander::IMPLICIT_COMMANDS {
        let mut entry = CatalogEntry::new(name, EntryKind::Control);
        entry.syntax = Syntax::Contextual;
        entry.sample = Some(match *name {
            "^" => "x^{2}".into(),
            "_" => "x_{i}".into(),
            _ => format!(r"\sum{name}_{{i=0}}^{{n}}i"),
        });
        entries.insert(name, entry);
    }
    for name in [r"\verb", r"\verb*"] {
        let mut entry = CatalogEntry::new(name, EntryKind::Control);
        entry.modes = Modes::Both;
        entry.syntax = Syntax::Verbatim;
        entry.arguments = vec![Argument::new(
            ArgumentKind::Value(ArgType::Raw),
            ArgumentShape::Delimited,
            "|text|",
        )];
        entry.sample = Some(format!("{name}|text|"));
        entries.insert(name, entry);
    }
    for entry in entries.values_mut() {
        if entry.kind == EntryKind::Symbol && entry.modes == Modes::Text {
            entry.sample = Some(format!(r"\text{{{}}}", entry.name));
        }
        if entry.visibility == Visibility::Internal {
            entry.sample = None;
        }
    }
    entries.into_values().collect()
}

fn replace(entries: &mut BTreeMap<&'static str, CatalogEntry>, mut entry: CatalogEntry) {
    if let Some(previous) = entries.remove(entry.name) {
        entry.symbols = previous.symbols;
    }
    entries.insert(entry.name, entry);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse;

    #[test]
    fn catalog_joins_aliases_modes_and_registry_sources() {
        assert!(catalog_all()
            .windows(2)
            .all(|pair| pair[0].name < pair[1].name));
        let alpha = find(r"\alpha").unwrap();
        assert_eq!(alpha.kind, EntryKind::Symbol);
        assert_eq!(alpha.modes, Modes::Math);
        assert_eq!(alpha.symbols[0].codepoint, Some('α'));
        assert_eq!(find(r"\textbf").unwrap().modes, Modes::Both);
        assert_eq!(find(r"\frac").unwrap().modes, Modes::Math);
        assert_eq!(find(r"\ne").unwrap().kind, EntryKind::Macro);
        assert_eq!(find(r"\neq").unwrap().kind, EntryKind::Macro);
        let sum = find(r"\sum").unwrap();
        assert_eq!(sum.kind, EntryKind::Function);
        assert_eq!(sum.symbols[0].codepoint, Some('∑'));
        assert_eq!(catalog().filter(|e| e.name == r"\quad").count(), 1);
        assert_eq!(find(r"\quad").unwrap().kind, EntryKind::Macro);
        assert_eq!(
            find(r"\operatorname").unwrap().arguments[1].kind,
            ArgumentKind::Value(ArgType::Text)
        );
        assert!(catalog().all(|e| e.visibility == Visibility::Public));
        assert_eq!(find(r"\@ifstar").unwrap().visibility, Visibility::Internal);
        assert!(find(r"\userMacro").is_none());
    }

    #[test]
    fn catalog_arguments_build_fraction_roots_and_typed_values() {
        let frac = find(r"\frac").unwrap();
        assert_eq!(frac.arguments.len(), 2);
        assert!(frac
            .arguments
            .iter()
            .all(|a| a.shape == ArgumentShape::Required));
        let root = find(r"\sqrt").unwrap();
        assert_eq!(root.arguments[0].shape, ArgumentShape::Optional);
        assert_eq!(root.arguments[0].sample, "3");
        assert_eq!(root.arguments[1].shape, ArgumentShape::Required);
        assert!(parse(r"\sqrt[3]{x}").is_ok());
        assert_eq!(
            find(r"\textcolor").unwrap().arguments[1].kind,
            ArgumentKind::Value(ArgType::Color)
        );
        assert_eq!(
            find(r"\href").unwrap().arguments[0].kind,
            ArgumentKind::Value(ArgType::Url)
        );
        assert_eq!(find(r"\blue").unwrap().arguments.len(), 1);
        assert_eq!(find(r"\Braket").unwrap().arguments.len(), 1);
        assert_eq!(
            find(r"\ce").unwrap().arguments[0].kind,
            ArgumentKind::Value(ArgType::Raw)
        );
    }

    #[test]
    fn catalog_special_forms_keep_delimiters_definitions_and_infix_syntax() {
        let left = find(r"\left").unwrap();
        assert!(left.primitive);
        assert_eq!(left.syntax, Syntax::Delimiter);
        assert_eq!(left.arguments[0].shape, ArgumentShape::Token);
        assert_eq!(left.arguments[0].kind, ArgumentKind::Delimiter);
        let over = find(r"\over").unwrap();
        assert!(over.infix);
        assert_eq!(over.syntax, Syntax::Infix);
        assert!(over.arguments.is_empty());
        let ast = parse(over.sample.as_ref().unwrap()).unwrap();
        let crate::ParseNode::OrdGroup { body, .. } = &ast[0] else {
            panic!("infix group")
        };
        assert!(matches!(body[0], crate::ParseNode::GenFrac { .. }));
        assert_eq!(
            find(r"\above").unwrap().arguments[0].kind,
            ArgumentKind::Value(ArgType::Size)
        );
        assert_eq!(find(r"\def").unwrap().syntax, Syntax::Definition);
        assert_eq!(
            find(r"\def").unwrap().arguments[0].kind,
            ArgumentKind::MacroName
        );
        assert_eq!(
            find(r"\char").unwrap().arguments[0].shape,
            ArgumentShape::Token
        );
        assert_eq!(find(r"\verb").unwrap().syntax, Syntax::Verbatim);
    }

    #[test]
    fn catalog_environment_samples_use_alignment_columns_and_counts() {
        let matrix = find("pmatrix*").unwrap();
        assert!(matrix.starred);
        assert_eq!(matrix.arguments[0].shape, ArgumentShape::Optional);
        assert_eq!(matrix.arguments[0].kind, ArgumentKind::Alignment);
        assert!(find("pmatrix").unwrap().arguments.is_empty());
        assert_eq!(
            find("array").unwrap().arguments[0].kind,
            ArgumentKind::Columns
        );
        assert_eq!(
            find("alignat*").unwrap().arguments[0].kind,
            ArgumentKind::Count
        );
        for entry in catalog().filter(|e| e.kind == EntryKind::Environment) {
            assert!(
                parse(entry.sample.as_ref().unwrap()).is_ok(),
                "{}",
                entry.name
            );
        }
        let ast = parse(r"\begin{pmatrix*}[r]x\end{pmatrix*}").unwrap();
        let crate::ParseNode::LeftRight { body, .. } = &ast[0] else {
            panic!("matrix delimiters")
        };
        let crate::ParseNode::Array {
            cols: Some(cols), ..
        } = &body[0]
        else {
            panic!("matrix body")
        };
        assert_eq!(cols[0].align.as_deref(), Some("r"));
    }

    #[test]
    fn catalog_samples_parse_as_complete_math() {
        let mut failures = Vec::new();
        for entry in catalog_all().iter().chain(builtin_macros()) {
            let Some(sample) = &entry.sample else {
                continue;
            };
            assert!(sample.len() <= 512, "{}", entry.name);
            if let Err(error) = parse(sample) {
                failures.push(format!("{}: {sample:?}: {}", entry.name, error.message));
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    #[test]
    fn catalog_reads_keep_parser_output_and_macro_scope() {
        let source = r"\def\local#1{\frac{#1}{2}}\local{x}+\sqrt[3]{y}";
        let before = serde_json::to_value(parse(source).unwrap()).unwrap();
        assert!(catalog().count() > 0);
        assert!(SourceTokens::new(source).next().is_some());
        assert_eq!(
            before,
            serde_json::to_value(parse(source).unwrap()).unwrap()
        );
        assert!(parse(r"\local{x}").is_err());
        assert!(find(r"\local").is_none());
    }
}
