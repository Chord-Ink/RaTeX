//! Read-only metadata for the shared built-in macro table.

use std::sync::LazyLock;

use super::{MacroDefinition, Template, BUILTINS};
use crate::editor::{
    Argument, ArgumentKind, ArgumentShape, CatalogEntry, EntryKind, Modes, Syntax, Visibility,
};
use crate::functions::ArgType;

static ENTRIES: LazyLock<Vec<CatalogEntry>> = LazyLock::new(|| {
    let mut entries: Vec<_> = BUILTINS
        .iter()
        .map(|(name, stored)| entry(name, &stored.definition))
        .collect();
    entries.sort_by_key(|entry| entry.name);
    entries
});

/// Built-in macros only, sorted by name and including internal helpers.
/// Handler syntax is recorded here alongside the macro table.
pub fn builtin_macros() -> &'static [CatalogEntry] {
    &ENTRIES
}

fn entry(name: &'static str, definition: &MacroDefinition) -> CatalogEntry {
    let mut entry = CatalogEntry::new(name, EntryKind::Macro);
    entry.modes = Modes::Contextual;
    let count = match definition {
        MacroDefinition::Text(text) => Some(Template::Text(text).num_args()),
        MacroDefinition::Tokens { num_args, .. } => Some(*num_args),
        MacroDefinition::Function(_) => handler_arity(name),
    };
    if let Some(count) = count {
        entry.arguments = (0..count)
            .map(|_| Argument::value(ArgType::Original, ArgumentShape::Required))
            .collect();
        entry.sample = Some(entry.argument_sample());
    } else {
        entry.syntax = Syntax::Contextual;
    }
    special(&mut entry);
    if entry.visibility == Visibility::Internal {
        entry.sample = None;
    }
    entry
}

fn handler_arity(name: &str) -> Option<usize> {
    match name {
        r"\dots" | r"\dotso" | r"\dotsc" | r"\cdots" => Some(0),
        r"\message" | r"\errmessage" | r"\ce" | r"\pu" => Some(1),
        r"\@firstoftwo" | r"\@secondoftwo" | r"\@ifstar" | r"\TextOrMath" | r"\htmlClass"
        | r"\htmlData" | r"\htmlId" => Some(2),
        r"\@ifnextchar" => Some(3),
        r"\bra@ket" | r"\bra@set" => Some(5),
        _ => None,
    }
}

fn special(entry: &mut CatalogEntry) {
    use ArgumentKind::*;
    use ArgumentShape::*;
    match entry.name {
        r"\Braket" | r"\set" | r"\Set" => {
            entry.arguments = vec![Argument::value(ArgType::Math, Required)];
            entry.sample = Some(entry.argument_sample());
        }
        r"\operatorname" => {
            entry.modes = Modes::Math;
            entry.arguments = vec![
                Argument::new(Value(ArgType::Raw), OptionalStar, "*"),
                Argument::value(ArgType::Text, Required),
            ];
            entry.syntax = Syntax::Contextual;
            entry.sample = Some(r"\operatorname{rank}".into());
        }
        r"\newcommand" | r"\renewcommand" | r"\providecommand" => {
            entry.syntax = Syntax::Definition;
            entry.modes = Modes::Both;
            entry.arguments = vec![
                Argument::new(MacroName, Required, r"\sample"),
                Argument::new(Count, Optional, "1"),
                Argument::new(Value(ArgType::Raw), Required, "#1"),
            ];
            entry.sample = Some(if entry.name == r"\renewcommand" {
                r"{\def\sample{x}\renewcommand{\sample}[1]{#1}\sample{y}}".into()
            } else {
                format!(r"{{{}{{\sample}}[1]{{#1}}\sample{{x}}}}", entry.name)
            });
        }
        r"\noexpand" => {
            entry.syntax = Syntax::Prefix;
            entry.arguments = vec![Argument::new(Value(ArgType::Raw), Token, r"\alpha")];
            entry.sample = Some(r"\noexpand\alpha".into());
        }
        r"\char" => {
            entry.syntax = Syntax::Prefix;
            entry.modes = Modes::Both;
            entry.arguments = vec![Argument::new(Number, Token, "65")];
            entry.sample = Some(r"\char65".into());
        }
        r"\ce" | r"\pu" => {
            entry.arguments = vec![Argument::new(
                Value(ArgType::Raw),
                Required,
                if entry.name == r"\ce" { "H2O" } else { "1 m" },
            )];
            entry.sample = Some(entry.argument_sample());
        }
        r"\TextOrMath" => {
            entry.modes = Modes::Both;
            entry.arguments = vec![
                Argument::value(ArgType::Text, Required),
                Argument::value(ArgType::Math, Required),
            ];
            entry.sample = Some(entry.argument_sample());
        }
        r"\htmlClass" | r"\htmlData" | r"\htmlId" => {
            entry.arguments[0].kind = Value(ArgType::Raw);
        }
        r"\message" | r"\errmessage" => {
            entry.arguments = vec![Argument::value(ArgType::Raw, Required)];
        }
        r"\bgroup" | r"\egroup" => {
            entry.syntax = Syntax::Contextual;
            entry.modes = Modes::Both;
            entry.sample = Some(r"\bgroup x\egroup".into());
        }
        r"\tmspace" => {
            entry.arguments[0].sample = "+";
            entry.arguments[1].sample = "3mu";
            entry.arguments[2].sample = "0.1em";
            entry.sample = Some(entry.argument_sample());
        }
        r"\newline" => {
            entry.syntax = Syntax::Contextual;
            entry.sample = Some(r"\begin{matrix}x\newline y\end{matrix}".into());
        }
        r"\llap" | r"\rlap" | r"\clap" | r"\underbar" => {
            entry.arguments = vec![Argument::value(ArgType::Text, Required)];
            entry.sample = Some(entry.argument_sample());
        }
        _ => {}
    }
}
