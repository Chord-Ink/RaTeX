# Editor data

`ratex-parser::editor` exposes source tokens and the built-in catalog. These
APIs use the lexer, function, environment, symbol and macro tables in RaTeX.
They do not need a successful parse or a font loader.

## Source tokens

```rust
use ratex_parser::editor::{SourceTokenKind, SourceTokens};

let source = "α\\frac  {x}% note\n\\verb|{x}%|";
for token in SourceTokens::new(source) {
    assert_eq!(token.text, &source[token.loc.start..token.loc.end]);
    if token.kind == SourceTokenKind::Comment {
        assert_eq!(token.text, "% note");
    }
}
```

`SourceTokens` borrows the input and allocates nothing. Its tokens cover every
byte once, with no gaps and no EOF token. Locations are UTF-8 byte offsets into
the exact input, including whitespace after commands. Callers can pause the
iterator between tokens and map those offsets into their own document.

Kinds cover commands, escapes, comments, whitespace, verbatim, open and close
brackets, scripts, alignment, math shifts, digit runs and other characters.
Characters keep trailing combining marks in U+0300–U+036F. The scanner uses
RaTeX's default lexical rules, without macro expansion or changed category
codes. Argument modes and command meanings come from the catalog and the
caller's context.

A `\verb` or `\verb*` command is a command token. The next token contains its
delimiters and body, with `Verbatim { closed: true }` when both delimiters are
present. An unfinished body reaches EOF with `closed: false`. Escapes and
comment marks inside that body remain verbatim. A trailing backslash, partial
command or unfinished group is valid input to the scanner.

The parser keeps using `Lexer::lex` and `Lexer::next_span`. Source tokens do not
change their whitespace, comment or macro rules.

## Catalog

- `catalog()` iterates public entries in name order.
- `catalog_all()` returns the same catalog with internal helpers included.
- `find(name)` looks up any entry, including internal helpers.
- `ratex_font::symbols::symbols()` iterates registered symbols in table order.
- `ratex_parser::macro_expander::editor::builtin_macros()` returns sorted
  built-in macro metadata, including internal helpers.

The catalog is built once and later calls borrow it. Each name occurs once.
Built-in macros take precedence over functions, which take precedence over
symbols, as in parsing. Each entry retains the registered symbol modes and
glyphs for its name. Unicode aliases and literal symbols are included; a
command menu can filter names beginning with a backslash.

An entry records its kind, visibility, modes, argument shapes, syntax form,
primitive/infix/starred flags, symbol data and an optional sample. Function
modes come from their registration. Macro modes can be `Contextual`: expansion
can depend on the current mode and arguments, so this does not promise that
arbitrary arguments work in both modes. Missing names may be user macros.

Arguments are in source order. Their kinds include RaTeX's `ArgType` values,
delimiters, column specs, alignment, counts and macro names. Shapes distinguish
required groups, optional square brackets, optional stars, single tokens,
delimited bodies and the rest of a special form. Argument samples are contents,
without surrounding argument brackets; a delimited body includes its chosen
delimiters.

Use `Syntax` before making a template. Only `Arguments` uses the ordinary
argument grammar. Infix commands need operands on both sides. `\left`,
`\middle`, `\right` and large delimiters take delimiter tokens. Definitions,
prefix commands and mode switches have their own forms. Environment entries
have a body between their begin and end markers. Starred matrix entries have
optional `[l]`, `[c]` or `[r]` alignment even though the parse registration has
zero optional arguments; that option is read by the matrix handler. Arrays
supply column specs and `alignat` supplies a column-pair count.

`sample` is a complete, bounded math input for previews, not insertion text.
Text-only entries are wrapped in `\text`. Public samples are checked by parsing
them. Internal helpers and entries without a complete sample return `None`.
The caller still applies its normal layout limits and handles missing fonts.
Samples are never run while building or looking up the catalog.

The editor metadata lives beside the function, environment and macro code.
When a handler reads arguments itself, update that metadata along with it.
The tests cover source offsets, partial input, registry aliases, modes, special
forms, environment arguments, sample parsing and unchanged parser state.
