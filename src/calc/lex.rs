// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Tokeniser: turns an expression into tokens, accepting the keyboard and
//! typographic spellings of each operator and the common LaTeX commands.

use super::CalcError;

/// A number as written: its digits (without a base prefix) and its base.
#[derive(Debug, Clone, PartialEq)]
pub struct NumLit {
    pub text: String,
    pub base: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Tok {
    Num(NumLit),
    Ident(String),
    /// A LaTeX command such as `\frac`, named without its backslash.
    Cmd(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Bang,
    Percent,
    Comma,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Pipe,
    Equals,
    Underscore,
    Degree,
    Prime,
    DoublePrime,
    Shl,
    Shr,
    Question,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub tok: Tok,
    /// Position of the token's first character, counted from 1.
    pub pos: usize,
}

pub fn lex(input: &str) -> Result<Vec<Token>, CalcError> {
    let chars: Vec<char> = input.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let pos = i + 1;
        let mut push = |tok: Tok| out.push(Token { tok, pos });
        match c {
            c if c.is_whitespace() => {
                i += 1;
            }
            '0'..='9' | '.' => {
                let (lit, next) = number(&chars, i)?;
                push(Tok::Num(lit));
                i = next;
            }
            c if c.is_alphabetic() || c == '_' && next_is_alpha(&chars, i) => {
                let start = i;
                while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                    i += 1;
                }
                push(Tok::Ident(chars[start..i].iter().collect()));
            }
            '\\' => {
                i += 1;
                if i < chars.len() && chars[i].is_ascii_alphabetic() {
                    let start = i;
                    while i < chars.len() && chars[i].is_ascii_alphabetic() {
                        i += 1;
                    }
                    push(Tok::Cmd(chars[start..i].iter().collect()));
                } else if i < chars.len() {
                    match chars[i] {
                        '{' => push(Tok::LParen),
                        '}' => push(Tok::RParen),
                        '%' => push(Tok::Percent),
                        ',' | ';' | '!' | ' ' | ':' => {}
                        other => {
                            return Err(CalcError::new(format!(
                                "unknown LaTeX symbol '\\{other}' at position {pos}"
                            )));
                        }
                    }
                    i += 1;
                } else {
                    return Err(CalcError::new(format!("a lone '\\' at position {pos}")));
                }
            }
            _ => {
                let (tok, len) = match (c, chars.get(i + 1).copied()) {
                    ('*', Some('*')) => (Tok::Caret, 2),
                    ('<', Some('<')) => (Tok::Shl, 2),
                    ('>', Some('>')) => (Tok::Shr, 2),
                    ('+', _) => (Tok::Plus, 1),
                    ('-' | '−' | '–', _) => (Tok::Minus, 1),
                    ('*' | '×' | '·' | '⋅' | '∙', _) => (Tok::Star, 1),
                    ('/' | '÷' | '∕', _) => (Tok::Slash, 1),
                    ('^', _) => (Tok::Caret, 1),
                    ('!', _) => (Tok::Bang, 1),
                    ('%', _) => (Tok::Percent, 1),
                    (',', _) => (Tok::Comma, 1),
                    ('(', _) => (Tok::LParen, 1),
                    (')', _) => (Tok::RParen, 1),
                    ('{', _) => (Tok::LBrace, 1),
                    ('}', _) => (Tok::RBrace, 1),
                    ('[', _) => (Tok::LBracket, 1),
                    (']', _) => (Tok::RBracket, 1),
                    ('|', _) => (Tok::Pipe, 1),
                    ('=', _) => (Tok::Equals, 1),
                    ('_', _) => (Tok::Underscore, 1),
                    ('°' | 'º', _) => (Tok::Degree, 1),
                    ('\'' | '′' | '’', _) => (Tok::Prime, 1),
                    ('"' | '″' | '”', _) => (Tok::DoublePrime, 1),
                    ('?', _) => (Tok::Question, 1),
                    ('π', _) => (Tok::Ident("pi".into()), 1),
                    ('√', _) => (Tok::Ident("sqrt".into()), 1),
                    ('∛', _) => (Tok::Ident("cbrt".into()), 1),
                    ('²', _) => {
                        push(Tok::Caret);
                        (Tok::Num(int_lit("2")), 1)
                    }
                    ('³', _) => {
                        push(Tok::Caret);
                        (Tok::Num(int_lit("3")), 1)
                    }
                    _ => {
                        return Err(CalcError::new(format!(
                            "unexpected character '{c}' at position {pos}"
                        )));
                    }
                };
                push(tok);
                i += len;
            }
        }
    }
    Ok(out)
}

fn next_is_alpha(chars: &[char], i: usize) -> bool {
    chars.get(i + 1).is_some_and(|c| c.is_alphabetic())
}

fn int_lit(s: &str) -> NumLit {
    NumLit {
        text: s.into(),
        base: 10,
    }
}

/// Reads a number at `i`: decimal (with an optional fraction, exponent and
/// thousands separators written as `,` only between groups of three digits is
/// not supported, since `,` separates arguments), or a `0x`, `0b`, `0o` literal.
fn number(chars: &[char], start: usize) -> Result<(NumLit, usize), CalcError> {
    let mut i = start;
    if chars[i] == '0'
        && let Some(&p) = chars.get(i + 1)
    {
        let base = match p {
            'x' | 'X' => 16,
            'b' | 'B' => 2,
            'o' | 'O' => 8,
            _ => 0,
        };
        if base != 0 && chars.get(i + 2).is_some_and(|c| c.is_digit(base)) {
            i += 2;
            let s = i;
            while i < chars.len() && (chars[i].is_digit(base) || chars[i] == '_') {
                i += 1;
            }
            if i < chars.len() && chars[i].is_ascii_alphanumeric() {
                let word: String = chars[start..]
                    .iter()
                    .take_while(|c| c.is_ascii_alphanumeric())
                    .collect();
                return Err(CalcError::new(format!(
                    "'{word}' at position {} is not a number in base {base}",
                    start + 1
                )));
            }
            let text: String = chars[s..i].iter().filter(|c| **c != '_').collect();
            return Ok((NumLit { text, base }, i));
        }
    }
    let s = i;
    let mut seen_dot = false;
    while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.' && !seen_dot) {
        if chars[i] == '.' {
            seen_dot = true;
        }
        i += 1;
    }
    // An exponent only where digits follow `e` or `E` (so `2e` is 2 times e).
    if i < chars.len() && (chars[i] == 'e' || chars[i] == 'E') {
        let mut j = i + 1;
        if j < chars.len() && (chars[j] == '+' || chars[j] == '-') {
            j += 1;
        }
        if j < chars.len() && chars[j].is_ascii_digit() {
            while j < chars.len() && chars[j].is_ascii_digit() {
                j += 1;
            }
            i = j;
        }
    }
    let text: String = chars[s..i].iter().collect();
    if text == "." {
        return Err(CalcError::new(format!(
            "a lone '.' at position {}",
            start + 1
        )));
    }
    Ok((NumLit { text, base: 10 }, i))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(s: &str) -> Vec<Tok> {
        lex(s).unwrap().into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn typographic_operators() {
        assert_eq!(toks("6 × 7"), toks("6 * 7"));
        assert_eq!(toks("8 ÷ 2"), toks("8 / 2"));
        assert_eq!(toks("5 − 3"), toks("5 - 3"));
        assert_eq!(toks("2 ** 10"), toks("2 ^ 10"));
    }

    #[test]
    fn numbers_and_bases() {
        assert_eq!(
            toks("0xFF3A"),
            vec![Tok::Num(NumLit {
                text: "FF3A".into(),
                base: 16
            })]
        );
        assert_eq!(
            toks("2.5e-3"),
            vec![Tok::Num(NumLit {
                text: "2.5e-3".into(),
                base: 10
            })]
        );
        assert_eq!(
            toks("2e"),
            vec![Tok::Num(int_lit("2")), Tok::Ident("e".into())]
        );
    }

    #[test]
    fn latex_commands() {
        assert_eq!(toks("\\frac{1}{2}")[0], Tok::Cmd("frac".into()));
        assert_eq!(toks("\\,")[..], []);
    }
}
