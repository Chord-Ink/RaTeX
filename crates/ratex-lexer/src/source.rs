//! Borrowed source tokens for editing incomplete TeX.

use crate::token::SourceLocation;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceTokenKind {
    Command,
    Escape,
    Comment,
    Whitespace,
    Verbatim { closed: bool },
    Open,
    Close,
    Script,
    Alignment,
    MathShift,
    Number,
    Character,
}

/// `text` is exactly the input at `loc`, in UTF-8 bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceToken<'a> {
    pub text: &'a str,
    pub loc: SourceLocation,
    pub kind: SourceTokenKind,
}

/// A lossless iterator with default TeX category codes and no macro expansion.
/// Every byte occurs once, including spaces after commands and comments.
/// It allocates nothing and can be paused between tokens. There is no EOF token.
#[derive(Debug, Clone)]
pub struct SourceTokens<'a> {
    source: &'a str,
    pos: usize,
    verbatim: bool,
}

impl<'a> SourceTokens<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            pos: 0,
            verbatim: false,
        }
    }

    fn take_while(&mut self, matches: impl Fn(char) -> bool) {
        while let Some(ch) = self.source[self.pos..].chars().next() {
            if !matches(ch) {
                break;
            }
            self.pos += ch.len_utf8();
        }
    }

    fn command(&mut self, start: usize) -> SourceTokenKind {
        let Some(ch) = self.source[self.pos..].chars().next() else {
            return SourceTokenKind::Command;
        };
        if !ch.is_ascii_alphabetic() && ch != '@' {
            self.pos += ch.len_utf8();
            return SourceTokenKind::Escape;
        }
        self.take_while(|c| c.is_ascii_alphabetic() || c == '@');
        if &self.source[start..self.pos] == r"\verb" {
            if self.source[self.pos..].starts_with('*') {
                self.pos += 1;
            }
            self.verbatim = true;
        }
        SourceTokenKind::Command
    }

    fn verbatim(&mut self, delimiter: char) -> SourceTokenKind {
        self.verbatim = false;
        for ch in self.source[self.pos..].chars() {
            self.pos += ch.len_utf8();
            if ch == delimiter {
                return SourceTokenKind::Verbatim { closed: true };
            }
        }
        SourceTokenKind::Verbatim { closed: false }
    }
}

impl<'a> Iterator for SourceTokens<'a> {
    type Item = SourceToken<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.pos;
        let ch = self.source[start..].chars().next()?;
        self.pos += ch.len_utf8();
        let kind = if self.verbatim {
            self.verbatim(ch)
        } else {
            match ch {
                '\\' => self.command(start),
                '%' => {
                    self.take_while(|c| !matches!(c, '\r' | '\n'));
                    SourceTokenKind::Comment
                }
                ' ' | '\t' | '\r' | '\n' => {
                    self.take_while(|c| matches!(c, ' ' | '\t' | '\r' | '\n'));
                    SourceTokenKind::Whitespace
                }
                '{' | '[' | '(' => SourceTokenKind::Open,
                '}' | ']' | ')' => SourceTokenKind::Close,
                '^' | '_' => SourceTokenKind::Script,
                '&' => SourceTokenKind::Alignment,
                '$' => SourceTokenKind::MathShift,
                '0'..='9' => {
                    self.take_while(|c| c.is_ascii_digit());
                    SourceTokenKind::Number
                }
                _ => {
                    self.take_while(|c| ('\u{0300}'..='\u{036f}').contains(&c));
                    SourceTokenKind::Character
                }
            }
        };
        Some(SourceToken {
            text: &self.source[start..self.pos],
            loc: SourceLocation {
                start,
                end: self.pos,
            },
            kind,
        })
    }
}

impl std::iter::FusedIterator for SourceTokens<'_> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Lexer;

    fn check_spans(source: &str) {
        let mut end = 0;
        for token in SourceTokens::new(source) {
            assert_eq!(token.loc.start, end);
            assert!(token.loc.end > end);
            assert_eq!(&source[token.loc.start..token.loc.end], token.text);
            assert_eq!(token.text.as_ptr(), source[end..].as_ptr());
            end = token.loc.end;
        }
        assert_eq!(end, source.len());
    }

    #[test]
    fn source_tokens_keep_comments_spaces_escapes_and_byte_offsets() {
        let source = "α\\frac \t{x}%中文\r\n\\%\\{\\}\\$\\ ";
        let tokens: Vec<_> = SourceTokens::new(source).collect();
        check_spans(source);
        assert_eq!(tokens.iter().map(|t| t.text).collect::<String>(), source);
        assert_eq!(tokens[1].loc, SourceLocation { start: 2, end: 7 });
        assert_eq!(tokens[2].text, " \t");
        assert_eq!(tokens[6].kind, SourceTokenKind::Comment);
        assert_eq!(tokens[6].text, "%中文");
        assert_eq!(tokens[7].text, "\r\n");
        assert!(tokens[8..]
            .iter()
            .all(|t| t.kind == SourceTokenKind::Escape));
    }

    #[test]
    fn source_verbatim_keeps_braces_comments_and_unicode_delimiters() {
        for (source, command, body) in [
            (r"\verb|{x}%\|", r"\verb", r"|{x}%\|"),
            (r"\verb*界{中文}%界", r"\verb*", "界{中文}%界"),
        ] {
            check_spans(source);
            let tokens: Vec<_> = SourceTokens::new(source).collect();
            assert_eq!(tokens.len(), 2);
            assert_eq!(tokens[0].text, command);
            assert_eq!(tokens[0].kind, SourceTokenKind::Command);
            assert_eq!(tokens[1].text, body);
            assert_eq!(tokens[1].kind, SourceTokenKind::Verbatim { closed: true });
        }
    }

    #[test]
    fn source_tokens_accept_every_incomplete_unicode_prefix() {
        let source = "é\u{0301}\\sqrt[12]{\\verb*界%{x}界\\foo@bar \\";
        for end in 0..=source.len() {
            if source.is_char_boundary(end) {
                check_spans(&source[..end]);
            }
        }
        let tokens: Vec<_> = SourceTokens::new("é\u{0301}\\verb|未完").collect();
        assert_eq!(tokens[0].text, "é\u{0301}");
        assert_eq!(tokens[2].kind, SourceTokenKind::Verbatim { closed: false });
        let mut empty = SourceTokens::new("");
        assert_eq!(empty.next(), None);
        assert_eq!(empty.next(), None);
    }

    #[test]
    fn source_scanning_keeps_parser_lexer_results() {
        let source = "\\alpha  %comment\n\\verb|x%|é\u{0301}\\%";
        let before = Lexer::new(source).lex_all();
        check_spans(source);
        assert_eq!(before, Lexer::new(source).lex_all());
        assert_eq!(
            before.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(),
            [r"\alpha", r"\verb|x%|", "é\u{0301}", r"\%", "EOF"]
        );
    }
}
