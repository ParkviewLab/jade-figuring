// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The engineering calculator: an expression in, its value and the
//! expression as read out.

mod eval;
mod functions;
mod lex;
mod parse;
mod render;
mod value;

#[cfg(test)]
mod tests;

/// Expressions longer than this are refused.
pub const MAX_EXPRESSION_CHARS: usize = 1_000;

pub use eval::TIME_LIMIT;
pub use value::{MAX_INT_DIGITS, MAX_SHOWN_DIGITS, SIG_DIGITS};

#[derive(Debug, Clone, PartialEq)]
pub struct CalcError {
    pub message: String,
}

impl CalcError {
    pub fn new(message: impl Into<String>) -> Self {
        CalcError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for CalcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// Evaluates `input`, returning `<expression as read> = <result>`.
pub fn calculate(input: &str) -> Result<String, CalcError> {
    let input = input.trim();
    let chars = input.chars().count();
    if chars > MAX_EXPRESSION_CHARS {
        return Err(CalcError::new(format!(
            "the expression is {chars} characters long; the limit is {MAX_EXPRESSION_CHARS}"
        )));
    }
    // A LaTeX expression may come wrapped in math delimiters.
    let input = input
        .trim_start_matches("$$")
        .trim_end_matches("$$")
        .trim_matches('$')
        .trim_start_matches("\\(")
        .trim_end_matches("\\)")
        .trim_start_matches("\\[")
        .trim_end_matches("\\]");
    let tokens = lex::lex(input)?;
    let tree = parse::parse(&tokens, chars)?;
    let value = eval::Eval::new().eval(&tree)?;
    Ok(format!(
        "{} = {}",
        render::render(&tree),
        value::show(&value)
    ))
}
