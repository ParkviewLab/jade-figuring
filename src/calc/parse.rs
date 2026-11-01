// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Pratt parser from tokens to an expression tree, reading calculator
//! notation, the word forms models write (`degrees`, `choose`, `to hex`) and
//! the common LaTeX commands.

use super::CalcError;
use super::lex::{NumLit, Tok, Token};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Mod,
    Choose,
    And,
    Or,
    Xor,
    Xnor,
    Nand,
    Shl,
    Shr,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Unit {
    Degrees,
    Radians,
    Gradians,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    Hex,
    Bin,
    Oct,
    Dec,
    Dms,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Num(NumLit),
    Ident(String),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    Factorial(Box<Expr>),
    Percent(Box<Expr>),
    Angle(Box<Expr>, Unit),
    /// Degrees, minutes and seconds: `12°31'45"`.
    Dms(Box<Expr>, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    Convert(Box<Expr>, Target),
}

// Binding powers, lowest first.
const BP_CONVERT: u8 = 1;
const BP_CHOOSE: u8 = 2;
const BP_OR: u8 = 3;
const BP_AND: u8 = 4;
const BP_SHIFT: u8 = 5;
const BP_ADD: u8 = 6;
const BP_MUL: u8 = 7;
const BP_NEG: u8 = 8;
const BP_POW: u8 = 9;

pub fn parse(tokens: &[Token], input_len: usize) -> Result<Expr, CalcError> {
    let mut tokens = tokens.to_vec();
    // A bare trailing `=` (as typed on a calculator) is harmless.
    if matches!(tokens.last(), Some(t) if t.tok == Tok::Equals) {
        tokens.pop();
    }
    if tokens.is_empty() {
        return Err(CalcError::new("the expression is empty"));
    }
    let mut p = Parser {
        toks: tokens,
        i: 0,
        input_len,
        depth: 0,
    };
    let e = p.expr(0)?;
    if let Some(t) = p.peek_token() {
        return Err(match t.tok {
            Tok::Equals => CalcError::new(format!(
                "the expression continues after '=' at position {}; send only the expression to calculate",
                t.pos
            )),
            Tok::RParen | Tok::RBrace | Tok::RBracket => CalcError::new(format!(
                "an unmatched closing bracket at position {}",
                t.pos
            )),
            Tok::Comma => CalcError::new(format!(
                "a ',' at position {} outside a function's brackets; write numbers without thousands separators (2340, not 2,340)",
                t.pos
            )),
            _ => CalcError::new(format!(
                "unexpected {} at position {}",
                describe(&t.tok),
                t.pos
            )),
        });
    }
    Ok(e)
}

struct Parser {
    toks: Vec<Token>,
    i: usize,
    input_len: usize,
    depth: usize,
}

/// Expressions nest at most this deep.
const MAX_DEPTH: usize = 200;

fn describe(t: &Tok) -> String {
    match t {
        Tok::Num(n) => format!("number '{}'", n.text),
        Tok::Ident(s) => format!("word '{s}'"),
        Tok::Cmd(s) => format!("'\\{s}'"),
        other => format!("'{}'", symbol(other)),
    }
}

fn symbol(t: &Tok) -> &'static str {
    match t {
        Tok::Plus => "+",
        Tok::Minus => "-",
        Tok::Star => "*",
        Tok::Slash => "/",
        Tok::Caret => "^",
        Tok::Bang => "!",
        Tok::Percent => "%",
        Tok::Comma => ",",
        Tok::LParen => "(",
        Tok::RParen => ")",
        Tok::LBrace => "{",
        Tok::RBrace => "}",
        Tok::LBracket => "[",
        Tok::RBracket => "]",
        Tok::Pipe => "|",
        Tok::Equals => "=",
        Tok::Underscore => "_",
        Tok::Degree => "°",
        Tok::Prime => "'",
        Tok::DoublePrime => "\"",
        Tok::Shl => "<<",
        Tok::Shr => ">>",
        Tok::Question => "?",
        _ => "?",
    }
}

fn unit_word(s: &str) -> Option<Unit> {
    match s.to_ascii_lowercase().as_str() {
        "deg" | "degree" | "degrees" => Some(Unit::Degrees),
        "rad" | "radian" | "radians" => Some(Unit::Radians),
        "grad" | "gradian" | "gradians" | "gon" => Some(Unit::Gradians),
        _ => None,
    }
}

fn target_word(s: &str) -> Option<Target> {
    match s.to_ascii_lowercase().as_str() {
        "hex" | "hexadecimal" | "base16" => Some(Target::Hex),
        "bin" | "binary" | "base2" => Some(Target::Bin),
        "oct" | "octal" | "base8" => Some(Target::Oct),
        "dec" | "decimal" | "base10" => Some(Target::Dec),
        "dms" => Some(Target::Dms),
        _ => None,
    }
}

fn infix_word(s: &str) -> Option<(BinOp, u8)> {
    Some(match s.to_ascii_lowercase().as_str() {
        "mod" => (BinOp::Mod, BP_MUL),
        "choose" => (BinOp::Choose, BP_CHOOSE),
        "and" => (BinOp::And, BP_AND),
        "nand" => (BinOp::Nand, BP_AND),
        "or" => (BinOp::Or, BP_OR),
        "xor" => (BinOp::Xor, BP_OR),
        "xnor" => (BinOp::Xnor, BP_OR),
        _ => return None,
    })
}

/// Words that never begin an operand, so never start an implied product.
fn is_keyword(s: &str) -> bool {
    unit_word(s).is_some()
        || infix_word(s).is_some()
        || matches!(
            s.to_ascii_lowercase().as_str(),
            "to" | "in" | "as" | "at" | "where"
        )
}

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.i).map(|t| &t.tok)
    }

    fn peek_at(&self, k: usize) -> Option<&Tok> {
        self.toks.get(self.i + k).map(|t| &t.tok)
    }

    fn peek_token(&self) -> Option<&Token> {
        self.toks.get(self.i)
    }

    fn pos(&self) -> usize {
        self.toks
            .get(self.i)
            .map(|t| t.pos)
            .unwrap_or(self.input_len + 1)
    }

    fn next(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.i).map(|t| t.tok.clone());
        self.i += 1;
        t
    }

    fn eat(&mut self, t: &Tok) -> bool {
        if self.peek() == Some(t) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn eat_ident(&mut self, word: &str) -> bool {
        if matches!(self.peek(), Some(Tok::Ident(s)) if s.eq_ignore_ascii_case(word)) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn expect_close(&mut self, open: &Tok, open_pos: usize) -> Result<(), CalcError> {
        let close = match open {
            Tok::LParen => Tok::RParen,
            Tok::LBrace => Tok::RBrace,
            Tok::LBracket => Tok::RBracket,
            _ => Tok::RParen,
        };
        // `(` may be closed by `)` or by an escaped `\}` (both lex as RParen),
        // and braces by `}`; a LaTeX group is not fussy about which.
        match self.peek() {
            Some(t)
                if *t == close
                    || matches!(t, Tok::RParen | Tok::RBrace)
                        && matches!(close, Tok::RParen | Tok::RBrace) =>
            {
                self.i += 1;
                Ok(())
            }
            _ => Err(CalcError::new(format!(
                "unbalanced parenthesis: the '{}' at position {open_pos} is not closed (expected '{}' at position {})",
                symbol(open),
                symbol(&close),
                self.pos()
            ))),
        }
    }

    /// A braced LaTeX group `{…}`, or a single token where LaTeX allows one (`\frac12`).
    fn group(&mut self) -> Result<Expr, CalcError> {
        let pos = self.pos();
        if self.eat(&Tok::LBrace) {
            let e = self.expr(0)?;
            self.expect_close(&Tok::LBrace, pos)?;
            Ok(e)
        } else if let Some(Tok::Num(n)) = self.peek().cloned() {
            // `\frac12` is ½: one digit per group.
            self.i += 1;
            if n.base == 10 && n.text.len() > 1 && n.text.chars().all(|c| c.is_ascii_digit()) {
                let (first, rest) = n.text.split_at(1);
                self.toks.insert(
                    self.i,
                    Token {
                        tok: Tok::Num(NumLit {
                            text: rest.into(),
                            base: 10,
                        }),
                        pos: pos + 1,
                    },
                );
                Ok(Expr::Num(NumLit {
                    text: first.into(),
                    base: 10,
                }))
            } else {
                Ok(Expr::Num(n))
            }
        } else {
            self.operand(BP_POW)
        }
    }

    fn expr(&mut self, min_bp: u8) -> Result<Expr, CalcError> {
        self.depth += 1;
        if self.depth > MAX_DEPTH {
            return Err(CalcError::new(format!(
                "the expression nests more than {MAX_DEPTH} levels deep"
            )));
        }
        let r = self.expr_inner(min_bp);
        self.depth -= 1;
        r
    }

    fn expr_inner(&mut self, min_bp: u8) -> Result<Expr, CalcError> {
        let mut lhs = self.prefix()?;
        while let Some(t) = self.peek().cloned() {
            let (op, bp, right_assoc) = match &t {
                Tok::Plus => (BinOp::Add, BP_ADD, false),
                Tok::Minus => (BinOp::Sub, BP_ADD, false),
                Tok::Star => (BinOp::Mul, BP_MUL, false),
                Tok::Slash => (BinOp::Div, BP_MUL, false),
                Tok::Caret => (BinOp::Pow, BP_POW, true),
                Tok::Shl => (BinOp::Shl, BP_SHIFT, false),
                Tok::Shr => (BinOp::Shr, BP_SHIFT, false),
                Tok::Cmd(c) if matches!(c.as_str(), "times" | "cdot" | "ast") => {
                    (BinOp::Mul, BP_MUL, false)
                }
                Tok::Cmd(c) if c == "div" => (BinOp::Div, BP_MUL, false),
                Tok::Cmd(c) if matches!(c.as_str(), "bmod" | "mod") => (BinOp::Mod, BP_MUL, false),
                Tok::Cmd(c) if c == "choose" => (BinOp::Choose, BP_CHOOSE, false),
                Tok::Cmd(c) if matches!(c.as_str(), "land" | "wedge") => {
                    (BinOp::And, BP_AND, false)
                }
                Tok::Cmd(c) if matches!(c.as_str(), "lor" | "vee") => (BinOp::Or, BP_OR, false),
                Tok::Cmd(c) if c == "oplus" => (BinOp::Xor, BP_OR, false),
                Tok::Ident(s) if matches!(s.to_ascii_lowercase().as_str(), "to" | "in" | "as") => {
                    if min_bp > BP_CONVERT {
                        break;
                    }
                    let word_pos = self.pos();
                    self.i += 1;
                    let target = match self.next() {
                        Some(Tok::Ident(w)) => target_word(&w),
                        _ => None,
                    };
                    match target {
                        Some(target) => {
                            lhs = Expr::Convert(Box::new(lhs), target);
                            continue;
                        }
                        None => {
                            return Err(CalcError::new(format!(
                                "after '{s}' at position {word_pos}, name a conversion: hex, binary, octal, decimal or dms"
                            )));
                        }
                    }
                }
                Tok::Ident(s) if infix_word(s).is_some() => {
                    let (op, bp) = infix_word(s).unwrap();
                    (op, bp, false)
                }
                // An implied product: `2pi`, `2(3)`, `(1+2)(3)`, `3 sin(x)`.
                Tok::Num(_) | Tok::LParen | Tok::LBrace | Tok::Pipe if self.starts_operand() => {
                    if BP_MUL < min_bp {
                        break;
                    }
                    if matches!(t, Tok::Num(_)) && matches!(lhs, Expr::Num(_) | Expr::Neg(_)) {
                        return Err(CalcError::new(format!(
                            "two numbers in a row at position {}; write one number without spaces, or put an operator between them",
                            self.pos()
                        )));
                    }
                    let rhs = self.expr(BP_MUL + 1)?;
                    lhs = Expr::Bin(BinOp::Mul, Box::new(lhs), Box::new(rhs));
                    continue;
                }
                Tok::Ident(s) if !is_keyword(s) => {
                    if BP_MUL < min_bp {
                        break;
                    }
                    let rhs = self.expr(BP_MUL + 1)?;
                    lhs = Expr::Bin(BinOp::Mul, Box::new(lhs), Box::new(rhs));
                    continue;
                }
                Tok::Cmd(c) if starts_operand_cmd(c) => {
                    if BP_MUL < min_bp {
                        break;
                    }
                    let rhs = self.expr(BP_MUL + 1)?;
                    lhs = Expr::Bin(BinOp::Mul, Box::new(lhs), Box::new(rhs));
                    continue;
                }
                _ => break,
            };
            if bp < min_bp {
                break;
            }
            let op_pos = self.pos();
            self.i += 1;
            // `x^\circ` and `x^{\circ}` are x degrees.
            if op == BinOp::Pow && self.eat_circ() {
                lhs = self.postfix(Expr::Angle(Box::new(lhs), Unit::Degrees))?;
                continue;
            }
            if self.peek().is_none() {
                return Err(CalcError::new(format!(
                    "the expression ends after '{}' at position {op_pos}",
                    symbol(&t)
                )));
            }
            let next_bp = if right_assoc { bp } else { bp + 1 };
            let rhs = if op == BinOp::Pow {
                self.operand_or_neg(next_bp)?
            } else {
                self.expr(next_bp)?
            };
            lhs = Expr::Bin(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    /// `\circ` or `{\circ}` after `^`.
    fn eat_circ(&mut self) -> bool {
        if matches!(self.peek(), Some(Tok::Cmd(c)) if c == "circ") {
            self.i += 1;
            return true;
        }
        if matches!(self.peek(), Some(Tok::LBrace))
            && matches!(self.peek_at(1), Some(Tok::Cmd(c)) if c == "circ")
            && matches!(self.peek_at(2), Some(Tok::RBrace | Tok::RParen))
        {
            self.i += 3;
            return true;
        }
        false
    }

    /// The right side of `^`, which may itself be negated: `2^-3`.
    fn operand_or_neg(&mut self, bp: u8) -> Result<Expr, CalcError> {
        if self.eat(&Tok::Minus) {
            let e = self.expr(bp)?;
            Ok(Expr::Neg(Box::new(e)))
        } else {
            self.expr(bp)
        }
    }

    fn starts_operand(&self) -> bool {
        match self.peek() {
            Some(Tok::Num(_) | Tok::LParen | Tok::LBrace) => true,
            Some(Tok::Ident(s)) => !is_keyword(s),
            Some(Tok::Cmd(c)) => starts_operand_cmd(c),
            _ => false,
        }
    }

    fn prefix(&mut self) -> Result<Expr, CalcError> {
        match self.peek() {
            Some(Tok::Minus) => {
                self.i += 1;
                let e = self.expr(BP_NEG)?;
                Ok(Expr::Neg(Box::new(e)))
            }
            Some(Tok::Plus) => {
                self.i += 1;
                self.expr(BP_NEG)
            }
            Some(Tok::Ident(s))
                if s.eq_ignore_ascii_case("not")
                    && !matches!(self.peek_at(1), Some(Tok::LParen)) =>
            {
                self.i += 1;
                let e = self.expr(BP_NEG)?;
                Ok(Expr::Not(Box::new(e)))
            }
            Some(Tok::Cmd(c)) if matches!(c.as_str(), "neg" | "lnot") => {
                self.i += 1;
                let e = self.expr(BP_NEG)?;
                Ok(Expr::Not(Box::new(e)))
            }
            _ => self.operand(BP_NEG),
        }
    }

    /// A primary followed by its postfix operators.
    fn operand(&mut self, _bp: u8) -> Result<Expr, CalcError> {
        let e = self.primary()?;
        self.postfix(e)
    }

    fn postfix(&mut self, mut e: Expr) -> Result<Expr, CalcError> {
        loop {
            match self.peek() {
                Some(Tok::Bang) => {
                    self.i += 1;
                    e = Expr::Factorial(Box::new(e));
                }
                Some(Tok::Percent) => {
                    self.i += 1;
                    e = Expr::Percent(Box::new(e));
                }
                Some(Tok::Degree) => {
                    self.i += 1;
                    e = self.dms_tail(e)?;
                }
                Some(Tok::Ident(s)) if unit_word(s).is_some() => {
                    let unit = unit_word(s).unwrap();
                    self.i += 1;
                    e = Expr::Angle(Box::new(e), unit);
                }
                // `52C5` and `8P3`.
                Some(Tok::Ident(s))
                    if matches!(e, Expr::Num(_))
                        && s.len() > 1
                        && matches!(s.as_bytes()[0], b'C' | b'P')
                        && s[1..].chars().all(|c| c.is_ascii_digit()) =>
                {
                    let name = if s.starts_with('C') { "nCr" } else { "nPr" };
                    let k = Expr::Num(NumLit {
                        text: s[1..].into(),
                        base: 10,
                    });
                    self.i += 1;
                    e = Expr::Call(name.into(), vec![e, k]);
                }
                _ => break,
            }
        }
        Ok(e)
    }

    /// After `°`: optional minutes `'` and seconds `"`.
    fn dms_tail(&mut self, deg: Expr) -> Result<Expr, CalcError> {
        let save = self.i;
        if let Some(Tok::Num(m)) = self.peek().cloned()
            && matches!(self.peek_at(1), Some(Tok::Prime))
        {
            self.i += 2;
            let minutes = Expr::Num(m);
            let mut seconds = Expr::Num(NumLit {
                text: "0".into(),
                base: 10,
            });
            if let Some(Tok::Num(s)) = self.peek().cloned()
                && matches!(self.peek_at(1), Some(Tok::DoublePrime))
            {
                self.i += 2;
                seconds = Expr::Num(s);
            }
            return Ok(Expr::Dms(
                Box::new(deg),
                Box::new(minutes),
                Box::new(seconds),
            ));
        }
        self.i = save;
        Ok(Expr::Angle(Box::new(deg), Unit::Degrees))
    }

    fn args(&mut self, open_pos: usize) -> Result<Vec<Expr>, CalcError> {
        // Called after the opening bracket.
        let mut args = Vec::new();
        if matches!(self.peek(), Some(Tok::RParen)) {
            self.i += 1;
            return Ok(args);
        }
        loop {
            args.push(self.expr(0)?);
            if self.eat(&Tok::Comma) {
                continue;
            }
            self.expect_close(&Tok::LParen, open_pos)?;
            return Ok(args);
        }
    }

    fn primary(&mut self) -> Result<Expr, CalcError> {
        let pos = self.pos();
        let Some(t) = self.next() else {
            return Err(CalcError::new(format!(
                "the expression ends where a number was expected (position {pos})"
            )));
        };
        match t {
            Tok::Num(n) => Ok(Expr::Num(n)),
            Tok::LParen | Tok::LBrace | Tok::LBracket => {
                let e = self.expr(0)?;
                self.expect_close(&t, pos)?;
                Ok(e)
            }
            Tok::Pipe => {
                let e = self.expr(0)?;
                if !self.eat(&Tok::Pipe) {
                    return Err(CalcError::new(format!(
                        "the '|' at position {pos} is not closed"
                    )));
                }
                Ok(Expr::Call("abs".into(), vec![e]))
            }
            Tok::Ident(name) => self.ident(name, pos),
            Tok::Cmd(c) => self.command(c, pos),
            Tok::Equals => Err(CalcError::new(format!(
                "'=' at position {pos}: send only the expression to calculate"
            ))),
            other => Err(CalcError::new(format!(
                "unexpected {} at position {pos}",
                describe(&other)
            ))),
        }
    }

    fn ident(&mut self, name: String, pos: usize) -> Result<Expr, CalcError> {
        // `log_2(8)` and `log_{2} 8`; the lexer keeps `_` inside a word.
        let lower = name.to_ascii_lowercase();
        if let Some(rest) = lower.strip_prefix("log_") {
            let base = if rest.is_empty() {
                self.group()?
            } else if rest.chars().all(|c| c.is_ascii_digit()) {
                Expr::Num(NumLit {
                    text: rest.into(),
                    base: 10,
                })
            } else {
                Expr::Ident(name[4..].to_string())
            };
            let arg = self.call_arg()?;
            return Ok(Expr::Call("log".into(), vec![arg, base]));
        }
        if name.eq_ignore_ascii_case("log") && self.eat(&Tok::Underscore) {
            let base = self.group()?;
            let arg = self.call_arg()?;
            return Ok(Expr::Call("log".into(), vec![arg, base]));
        }
        if name.eq_ignore_ascii_case("not") {
            let open = self.pos();
            if self.eat(&Tok::LParen) {
                let mut a = self.args(open)?;
                if a.len() == 1 {
                    return Ok(Expr::Not(Box::new(a.remove(0))));
                }
                return Ok(Expr::Call("not".into(), a));
            }
        }
        let open = self.pos();
        if self.eat(&Tok::LParen) {
            let args = self.args(open)?;
            return Ok(Expr::Call(name, args));
        }
        if super::functions::is_function(&name) && self.eat(&Tok::Caret) {
            // `sin^2(x)` is (sin x)^2.
            let power = self.group()?;
            let open = self.pos();
            let call = if self.eat(&Tok::LParen) {
                Expr::Call(name, self.args(open)?)
            } else {
                Expr::Call(name, vec![self.call_arg()?])
            };
            return Ok(Expr::Bin(BinOp::Pow, Box::new(call), Box::new(power)));
        }
        if super::functions::is_function(&name) {
            // A function written without brackets: `ln 50`, `sin 30°`.
            if self.peek().is_none() {
                return Err(CalcError::new(format!(
                    "'{name}' at position {pos} needs an argument"
                )));
            }
            let arg = self.call_arg()?;
            return Ok(Expr::Call(name, vec![arg]));
        }
        Ok(Expr::Ident(name))
    }

    /// The argument of a function written without brackets.
    fn call_arg(&mut self) -> Result<Expr, CalcError> {
        let open = self.pos();
        if self.eat(&Tok::LParen) {
            let e = self.expr(0)?;
            self.expect_close(&Tok::LParen, open)?;
            return self.postfix(e);
        }
        if self.eat(&Tok::LBrace) {
            let e = self.expr(0)?;
            self.expect_close(&Tok::LBrace, open)?;
            return self.postfix(e);
        }
        // `sin 2x` reads as sin(2x), as calculators and LaTeX do.
        let first = self.operand(BP_POW)?;
        let mut e = first;
        if let Some(Tok::Caret) = self.peek() {
            self.i += 1;
            if self.eat_circ() {
                e = self.postfix(Expr::Angle(Box::new(e), Unit::Degrees))?;
            } else {
                let r = self.operand_or_neg(BP_POW)?;
                e = Expr::Bin(BinOp::Pow, Box::new(e), Box::new(r));
            }
        }
        while matches!(self.peek(), Some(Tok::Ident(s)) if !is_keyword(s) && !super::functions::is_function(s))
        {
            let r = self.operand(BP_POW)?;
            e = Expr::Bin(BinOp::Mul, Box::new(e), Box::new(r));
        }
        Ok(e)
    }

    fn command(&mut self, c: String, pos: usize) -> Result<Expr, CalcError> {
        match c.as_str() {
            "frac" | "dfrac" | "tfrac" => {
                let num = self.group()?;
                let den = self.group()?;
                if is_d(&num)
                    && let Some(var) = d_var(&den)
                {
                    return self.derivative(var, pos);
                }
                Ok(Expr::Bin(BinOp::Div, Box::new(num), Box::new(den)))
            }
            "sqrt" => {
                if self.eat(&Tok::LBracket) {
                    let n = self.expr(0)?;
                    self.expect_close(&Tok::LBracket, pos)?;
                    let x = self.group()?;
                    Ok(Expr::Call("root".into(), vec![x, n]))
                } else {
                    let x = self.group()?;
                    Ok(Expr::Call("sqrt".into(), vec![x]))
                }
            }
            "binom" | "dbinom" | "tbinom" => {
                let n = self.group()?;
                let k = self.group()?;
                Ok(Expr::Call("nCr".into(), vec![n, k]))
            }
            "left" | "right" | "big" | "Big" | "bigg" | "Bigg" => {
                // The delimiter that follows is what counts.
                if self.eat(&Tok::LParen) || self.eat(&Tok::LBracket) || self.eat(&Tok::LBrace) {
                    let e = self.expr(0)?;
                    self.eat(&Tok::Cmd("right".into()));
                    if !(self.eat(&Tok::RParen)
                        || self.eat(&Tok::RBracket)
                        || self.eat(&Tok::RBrace))
                    {
                        return Err(CalcError::new(format!(
                            "the '\\{c}' bracket at position {pos} is not closed"
                        )));
                    }
                    Ok(e)
                } else if self.eat(&Tok::Pipe) {
                    let e = self.expr(0)?;
                    self.eat(&Tok::Cmd("right".into()));
                    self.eat(&Tok::Pipe);
                    Ok(Expr::Call("abs".into(), vec![e]))
                } else {
                    self.primary()
                }
            }
            "lvert" | "vert" => self.closed_by(pos, &["rvert", "vert"], "abs"),
            "lfloor" => self.closed_by(pos, &["rfloor"], "floor"),
            "lceil" => self.closed_by(pos, &["rceil"], "ceil"),
            "pi" | "Pi" => Ok(Expr::Ident("pi".into())),
            "tau" => Ok(Expr::Ident("tau".into())),
            "phi" | "varphi" => Ok(Expr::Ident("phi".into())),
            "infty" => Err(CalcError::new(format!(
                "infinity at position {pos} cannot be calculated with"
            ))),
            "mathrm" | "operatorname" | "mathit" | "textrm" | "mathbf" => {
                let open = self.pos();
                if !self.eat(&Tok::LBrace) {
                    return Err(CalcError::new(format!(
                        "'\\{c}' at position {pos} needs a {{name}}"
                    )));
                }
                let mut name = String::new();
                while let Some(Tok::Ident(s)) = self.peek().cloned() {
                    name.push_str(&s);
                    self.i += 1;
                }
                self.expect_close(&Tok::LBrace, open)?;
                self.ident(name, pos)
            }
            "text" | "textbf" | "textit" | "mbox" => Err(CalcError::new(format!(
                "text in '\\{c}{{…}}' at position {pos} is not evaluated; send only the expression to calculate"
            ))),
            "sum" | "prod" => self.big_operator(&c, pos),
            "int" => self.integral(pos),
            "circ" => Err(CalcError::new(format!(
                "'\\circ' at position {pos} marks degrees after '^', as in 30^\\circ"
            ))),
            "cdot" | "times" | "div" => Err(CalcError::new(format!(
                "'\\{c}' at position {pos} needs a number before it"
            ))),
            name if super::functions::is_function(name) => {
                if name == "log" && self.eat(&Tok::Underscore) {
                    let base = self.group()?;
                    let arg = self.call_arg()?;
                    return Ok(Expr::Call("log".into(), vec![arg, base]));
                }
                // `\sin^2 x` is (sin x)^2.
                let mut power = None;
                if self.eat(&Tok::Caret) {
                    power = Some(self.group()?);
                }
                let open = self.pos();
                let call = if self.eat(&Tok::LParen) {
                    let args = self.args(open)?;
                    Expr::Call(name.into(), args)
                } else {
                    let arg = self.call_arg()?;
                    Expr::Call(name.into(), vec![arg])
                };
                Ok(match power {
                    Some(p) => Expr::Bin(BinOp::Pow, Box::new(call), Box::new(p)),
                    None => call,
                })
            }
            other => Err(CalcError::new(format!(
                "the LaTeX command '\\{other}' at position {pos} is not one this calculator knows"
            ))),
        }
    }

    fn closed_by(&mut self, pos: usize, closers: &[&str], func: &str) -> Result<Expr, CalcError> {
        let e = self.expr(0)?;
        match self.peek() {
            Some(Tok::Cmd(c)) if closers.contains(&c.as_str()) => {
                self.i += 1;
                Ok(Expr::Call(func.into(), vec![e]))
            }
            _ => Err(CalcError::new(format!(
                "the bracket opened at position {pos} is not closed with '\\{}'",
                closers[0]
            ))),
        }
    }

    /// `\sum_{k=1}^{n} body` and `\prod_{k=1}^{n} body`.
    fn big_operator(&mut self, c: &str, pos: usize) -> Result<Expr, CalcError> {
        if !self.eat(&Tok::Underscore) {
            return Err(CalcError::new(format!(
                "'\\{c}' at position {pos} needs its range, as in \\{c}_{{k=1}}^{{10}}"
            )));
        }
        let open = self.pos();
        let braced = self.eat(&Tok::LBrace);
        let var = match self.next() {
            Some(Tok::Ident(v)) => v,
            _ => {
                return Err(CalcError::new(format!(
                    "'\\{c}' at position {pos} needs a variable, as in \\{c}_{{k=1}}^{{10}}"
                )));
            }
        };
        if !self.eat(&Tok::Equals) {
            return Err(CalcError::new(format!(
                "'\\{c}' at position {pos} needs a starting value, as in \\{c}_{{k=1}}^{{10}}"
            )));
        }
        let from = if braced {
            let e = self.expr(0)?;
            self.expect_close(&Tok::LBrace, open)?;
            e
        } else {
            self.operand(BP_POW)?
        };
        if !self.eat(&Tok::Caret) {
            return Err(CalcError::new(format!(
                "'\\{c}' at position {pos} needs its upper end, as in \\{c}_{{k=1}}^{{10}}"
            )));
        }
        let to = self.group()?;
        let body = self.expr(BP_MUL)?;
        let name = if c == "sum" { "sum" } else { "prod" };
        Ok(Expr::Call(
            name.into(),
            vec![body, Expr::Ident(var), from, to],
        ))
    }

    /// `\int_{a}^{b} f \, dx`.
    fn integral(&mut self, pos: usize) -> Result<Expr, CalcError> {
        if !self.eat(&Tok::Underscore) {
            return Err(CalcError::new(format!(
                "'\\int' at position {pos} needs its bounds, as in \\int_{{0}}^{{1}} x^2 \\, dx"
            )));
        }
        let a = self.group()?;
        if !self.eat(&Tok::Caret) {
            return Err(CalcError::new(format!(
                "'\\int' at position {pos} needs an upper bound, as in \\int_{{0}}^{{1}} x^2 \\, dx"
            )));
        }
        let b = self.group()?;
        // The integrand runs to the differential `dx` (or `d x`).
        let start = self.i;
        let mut end = None;
        let mut var = None;
        let mut j = self.i;
        while j < self.toks.len() {
            match &self.toks[j].tok {
                Tok::Ident(s)
                    if s.len() >= 2
                        && s.starts_with('d')
                        && s[1..].chars().all(|c| c.is_alphabetic())
                        && s.len() == 2 =>
                {
                    end = Some(j);
                    var = Some(s[1..].to_string());
                    break;
                }
                Tok::Ident(s) if s == "d" => {
                    if let Some(Tok::Ident(v)) = self.toks.get(j + 1).map(|t| &t.tok) {
                        end = Some(j);
                        var = Some(v.clone());
                        break;
                    }
                }
                _ => {}
            }
            j += 1;
        }
        let (Some(end), Some(var)) = (end, var) else {
            return Err(CalcError::new(format!(
                "'\\int' at position {pos} needs its differential, as in \\int_{{0}}^{{1}} x^2 \\, dx"
            )));
        };
        let mut sub = Parser {
            toks: self.toks[start..end].to_vec(),
            i: 0,
            input_len: self.input_len,
            depth: self.depth,
        };
        let f = sub.expr(0)?;
        if let Some(t) = sub.peek_token() {
            return Err(CalcError::new(format!(
                "unexpected {} at position {} in the integrand",
                describe(&t.tok),
                t.pos
            )));
        }
        self.i = end
            + if self.toks[end].tok == Tok::Ident("d".into()) {
                2
            } else {
                1
            };
        Ok(Expr::Call(
            "integral".into(),
            vec![f, Expr::Ident(var), a, b],
        ))
    }

    /// After `\frac{d}{dx}`: the function, then the point (`|_{x=a}` or `at x = a`).
    fn derivative(&mut self, var: String, pos: usize) -> Result<Expr, CalcError> {
        let f = self.expr(BP_MUL)?;
        if matches!(self.peek(), Some(Tok::Cmd(c)) if matches!(c.as_str(), "big" | "Big" | "bigg" | "Bigg" | "right"))
        {
            self.i += 1;
        }
        let at = if self.eat(&Tok::Pipe) {
            if !self.eat(&Tok::Underscore) {
                return Err(self.no_point(pos));
            }
            let open = self.pos();
            let braced = self.eat(&Tok::LBrace);
            self.eat_ident(&var);
            if !self.eat(&Tok::Equals) {
                return Err(self.no_point(pos));
            }
            let a = self.expr(0)?;
            if braced {
                self.expect_close(&Tok::LBrace, open)?;
            }
            a
        } else if self.eat_ident("at") || self.eat_ident("where") {
            self.eat_ident(&var);
            if !self.eat(&Tok::Equals) {
                return Err(self.no_point(pos));
            }
            self.expr(BP_ADD)?
        } else {
            return Err(self.no_point(pos));
        };
        Ok(Expr::Call("deriv".into(), vec![f, Expr::Ident(var), at]))
    }

    fn no_point(&self, pos: usize) -> CalcError {
        CalcError::new(format!(
            "the derivative at position {pos} needs a point, as in \\frac{{d}}{{dx}} x^2 \\Big|_{{x=3}} or deriv(x^2, x, 3)"
        ))
    }
}

fn starts_operand_cmd(c: &str) -> bool {
    matches!(
        c,
        "frac"
            | "dfrac"
            | "tfrac"
            | "sqrt"
            | "binom"
            | "pi"
            | "tau"
            | "phi"
            | "left"
            | "lvert"
            | "lfloor"
            | "lceil"
            | "mathrm"
            | "operatorname"
            | "sum"
            | "prod"
            | "int"
            | "text"
            | "textbf"
            | "textit"
            | "mbox"
    ) || super::functions::is_function(c)
}

fn is_d(e: &Expr) -> bool {
    matches!(e, Expr::Ident(s) if s == "d")
}

fn d_var(e: &Expr) -> Option<String> {
    match e {
        Expr::Ident(s) if s.len() == 2 && s.starts_with('d') => Some(s[1..].into()),
        Expr::Bin(BinOp::Mul, a, b) if is_d(a) => match b.as_ref() {
            Expr::Ident(v) => Some(v.clone()),
            _ => None,
        },
        _ => None,
    }
}
