use super::{ArgType, FunctionSpec};
use crate::editor::{
    Argument, ArgumentKind, ArgumentShape, CatalogEntry, EntryKind, Modes, Syntax,
};

pub(crate) fn entry(name: &'static str, spec: &FunctionSpec) -> CatalogEntry {
    let mut entry = CatalogEntry::new(name, EntryKind::Function);
    entry.modes = match (spec.allowed_in_math, spec.allowed_in_text) {
        (true, true) => Modes::Both,
        (true, false) => Modes::Math,
        (false, true) => Modes::Text,
        _ => Modes::Contextual,
    };
    entry.primitive = spec.primitive;
    entry.infix = spec.infix;
    entry.arguments = (0..spec.num_args + spec.num_optional_args)
        .map(|index| {
            let kind = spec
                .arg_types
                .as_ref()
                .and_then(|types| types.get(index))
                .copied()
                .unwrap_or(if spec.primitive {
                    ArgType::Primitive
                } else {
                    ArgType::Original
                });
            let shape = if index < spec.num_optional_args {
                ArgumentShape::Optional
            } else if kind == ArgType::Primitive {
                ArgumentShape::Token
            } else {
                ArgumentShape::Required
            };
            Argument::value(kind, shape)
        })
        .collect();
    if spec.infix {
        entry.syntax = Syntax::Infix;
        entry.sample = Some(format!("{{a {} b}}", entry.argument_sample()));
        return entry;
    }
    match spec.node_type {
        "leftright" | "leftright-right" | "middle" | "delimsizing" => {
            entry.syntax = Syntax::Delimiter;
            entry.arguments = vec![Argument::new(
                ArgumentKind::Delimiter,
                ArgumentShape::Token,
                "(",
            )];
            entry.sample = Some(match name {
                r"\left" | r"\right" => r"\left(x\right)".into(),
                r"\middle" => r"\left(x\middle|y\right)".into(),
                _ => format!("{name}("),
            });
        }
        "styling" | "sizing" if spec.allowed_in_math => {
            entry.syntax = Syntax::Switch;
            entry.sample = Some(format!("{{{name} x}}"));
        }
        "font" if spec.num_args == 0 => {
            entry.syntax = Syntax::Switch;
            entry.sample = Some(format!("{{{name} x}}"));
        }
        _ => entry.sample = Some(entry.argument_sample()),
    }
    special(&mut entry);
    if entry.modes == Modes::Text {
        entry.sample = entry.sample.map(|sample| format!(r"\text{{{sample}}}"));
    }
    entry
}

fn special(entry: &mut CatalogEntry) {
    use ArgumentKind::*;
    use ArgumentShape::*;
    match entry.name {
        r"\def" | r"\gdef" | r"\edef" | r"\xdef" => {
            entry.syntax = Syntax::Definition;
            entry.arguments = vec![
                Argument::new(MacroName, Token, r"\sample"),
                Argument::new(Parameters, Rest, "#1"),
                Argument::value(ArgType::Raw, Required),
            ];
            entry.sample = Some(format!(r"{{{}\sample#1{{#1}}\sample{{x}}}}", entry.name));
        }
        r"\let" | r"\futurelet" | r"\global" | r"\long" => {
            entry.syntax = Syntax::Prefix;
            entry.arguments = match entry.name {
                r"\let" => vec![
                    Argument::new(MacroName, Token, r"\sample"),
                    Argument::value(ArgType::Raw, Token),
                ],
                r"\futurelet" => vec![
                    Argument::new(MacroName, Token, r"\sample"),
                    Argument::value(ArgType::Raw, Token),
                    Argument::value(ArgType::Raw, Token),
                ],
                _ => vec![Argument::new(Value(ArgType::Raw), Rest, r"\def\sample{x}")],
            };
            entry.sample = Some(match entry.name {
                r"\let" => r"{\let\sample=x\sample}".into(),
                r"\futurelet" => r"{\futurelet\sample xy}".into(),
                _ => format!(r"{{{}\def\sample{{x}}\sample}}", entry.name),
            });
        }
        r"\begin" | r"\end" => {
            entry.syntax = Syntax::Environment;
            entry.arguments = vec![Argument::new(EnvironmentName, Required, "matrix")];
            entry.sample = Some(r"\begin{matrix}x\end{matrix}".into());
        }
        r"\color" => {
            entry.syntax = Syntax::Switch;
            entry.sample = Some(r"{\color{blue}x}".into());
        }
        r"\hline" | r"\hdashline" => {
            entry.syntax = Syntax::Contextual;
            entry.sample = Some(format!(
                r"\begin{{array}}{{c}}{} x\end{{array}}",
                entry.name
            ));
        }
        r"\genfrac" => {
            entry.arguments[0].sample = "(";
            entry.arguments[1].sample = ")";
            entry.arguments[2].sample = "0.04em";
            entry.arguments[3].sample = "0";
            entry.sample = Some(entry.argument_sample());
        }
        r"\sqrt" => {
            entry.arguments[0].sample = "3";
        }
        r"\mkern" | r"\mskip" => {
            entry.arguments[0].sample = "3mu";
            entry.sample = Some(entry.argument_sample());
        }
        r"\tag" => {
            entry.syntax = Syntax::Contextual;
            entry.arguments = vec![
                Argument::new(Value(ArgType::Raw), OptionalStar, "*"),
                Argument::new(Value(ArgType::Original), Token, "1"),
            ];
            entry.sample = Some(r"x\tag{1}".into());
        }
        r"\\" | r"\newline" => {
            entry.syntax = Syntax::Contextual;
            entry.arguments = vec![Argument::new(Value(ArgType::Size), Optional, "1em")];
            entry.sample = Some(format!(r"\begin{{matrix}}x{} y\end{{matrix}}", entry.name));
        }
        r"\(" | "$" => {
            entry.syntax = Syntax::Switch;
            entry.sample = Some(if entry.name == "$" {
                "$x$".into()
            } else {
                r"\(x\)".into()
            });
        }
        r"\)" | r"\]" => {
            entry.syntax = Syntax::Contextual;
            entry.sample = (entry.name == r"\)").then(|| r"\(x\)".into());
        }
        _ => {}
    }
}
