// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Evaluation of an expression tree, with a step budget checked as it runs,
//! since a blocking calculation cannot be stopped from outside.

use std::f64::consts::PI;
use std::time::{Duration, Instant};

use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};

use super::CalcError;
use super::functions::{self, canonical};
use super::lex::NumLit;
use super::parse::{BinOp, Expr, Target, Unit};
use super::value::{MAX_INT_DIGITS, Value, group};

/// A calculation is stopped after this long.
pub const TIME_LIMIT: Duration = Duration::from_secs(2);
/// Sums and products run over at most this many terms.
pub const MAX_TERMS: i64 = 100_000;

pub struct Eval {
    started: Instant,
    steps: u64,
    vars: Vec<(String, Value)>,
}

impl Eval {
    pub fn new() -> Self {
        Eval {
            started: Instant::now(),
            steps: 0,
            vars: Vec::new(),
        }
    }

    fn tick(&mut self) -> Result<(), CalcError> {
        self.steps += 1;
        if self.steps.is_multiple_of(1024) && self.started.elapsed() > TIME_LIMIT {
            return Err(CalcError::new(format!(
                "the calculation took longer than {} seconds and was stopped",
                TIME_LIMIT.as_secs()
            )));
        }
        Ok(())
    }

    pub fn eval(&mut self, e: &Expr) -> Result<Value, CalcError> {
        let v = self.eval_node(e)?;
        if let Value::Real(x) = v {
            if x.is_nan() {
                return Err(CalcError::new("the result is undefined"));
            }
            if x.is_infinite() {
                return Err(CalcError::new(
                    "the result is too large (beyond about 1.8 × 10^308)",
                ));
            }
        }
        Ok(v)
    }

    fn eval_node(&mut self, e: &Expr) -> Result<Value, CalcError> {
        self.tick()?;
        match e {
            Expr::Num(n) => number(n),
            Expr::Ident(name) => self.ident(name),
            Expr::Neg(x) => Ok(match self.eval(x)? {
                Value::Int(i) => Value::Int(-i),
                v => Value::Real(-real(&v)?),
            }),
            Expr::Not(x) => {
                let v = self.eval(x)?;
                functions::bitnot(&int_of(&v, "not")?, None)
            }
            Expr::Factorial(x) => {
                let v = self.eval(x)?;
                match v.as_int() {
                    Some(n) => functions::factorial(&n),
                    None => Err(CalcError::new(format!(
                        "the factorial needs a whole number, not {}",
                        super::value::show(&v)
                    ))),
                }
            }
            Expr::Percent(x) => Ok(Value::Real(real(&self.eval(x)?)? / 100.0)),
            Expr::Angle(x, unit) => {
                let v = real(&self.eval(x)?)?;
                Ok(Value::Real(match unit {
                    Unit::Degrees => v * PI / 180.0,
                    Unit::Radians => v,
                    Unit::Gradians => v * PI / 200.0,
                }))
            }
            Expr::Dms(d, m, s) => {
                let deg = real(&self.eval(d)?)?;
                let min = real(&self.eval(m)?)?;
                let sec = real(&self.eval(s)?)?;
                let sign = if deg < 0.0 { -1.0 } else { 1.0 };
                Ok(Value::Real(
                    sign * (deg.abs() + min / 60.0 + sec / 3600.0) * PI / 180.0,
                ))
            }
            Expr::Bin(op, a, b) => self.binary(*op, a, b),
            Expr::Call(name, args) => self.call(name, args),
            Expr::Convert(x, target) => self.convert(x, *target),
        }
    }

    fn ident(&mut self, name: &str) -> Result<Value, CalcError> {
        if let Some((_, v)) = self.vars.iter().rev().find(|(n, _)| n == name) {
            return Ok(v.clone());
        }
        if let Some(c) = functions::constant(name) {
            return Ok(Value::Real(c));
        }
        if functions::canonical(name).is_some() && name.len() > 1 {
            return Err(CalcError::new(format!(
                "'{name}' is a function and needs an argument, as in {name}(x)"
            )));
        }
        if matches!(name, "i" | "j") {
            return Err(CalcError::new(
                "complex numbers are not supported in this version",
            ));
        }
        Err(match functions::suggest(name) {
            Some(s) => CalcError::new(format!("unknown name '{name}'; use {s}")),
            None => CalcError::new(format!(
                "unknown name '{name}': a variable has a value only inside sum, prod, deriv and integral"
            )),
        })
    }

    fn binary(&mut self, op: BinOp, a: &Expr, b: &Expr) -> Result<Value, CalcError> {
        if matches!(op, BinOp::Add | BinOp::Sub) {
            let leading = match a {
                Expr::Percent(p) => Some(p),
                Expr::Neg(inner) => match inner.as_ref() {
                    Expr::Percent(p) => Some(p),
                    _ => None,
                },
                _ => None,
            };
            if let Some(p) = leading {
                let p = super::render::render(p);
                return Err(CalcError::new(format!(
                    "{p}% of what? Write the value first, as in 100 + {p}%, or write {p}% as a fraction"
                )));
            }
            if let Expr::Percent(p) = b {
                // A percentage added to a value is a percentage of that value.
                let base = self.eval(a)?;
                let p = real(&self.eval(p)?)? / 100.0;
                let f = if op == BinOp::Add { 1.0 + p } else { 1.0 - p };
                return Ok(Value::Real(real(&base)? * f));
            }
        }
        let x = self.eval(a)?;
        let y = self.eval(b)?;
        arith(op, &x, &y)
    }

    fn call(&mut self, name: &str, args: &[Expr]) -> Result<Value, CalcError> {
        // `hex_to_bin(0xFF3A)`, `dec_to_hex(255)`, `hex2dec(FF)`.
        if let Some((from, to)) = base_pair(name) {
            if args.len() != 1 {
                return Err(CalcError::new(format!("{name} takes one number")));
            }
            let i = self.number_in_base(&args[0], from)?;
            return Ok(match to {
                10 => Value::Int(i),
                b => Value::Text(functions::to_base(&i, b)),
            });
        }
        let Some(canon) = canonical(name) else {
            return Err(match functions::suggest(name) {
                Some(s) => CalcError::new(format!("unknown function '{name}'; use {s}")),
                None => CalcError::new(format!("unknown function '{name}'")),
            });
        };
        match canon {
            "sum" | "prod" => self.series(canon, args),
            "deriv" => self.derivative(args),
            "integral" => self.integral(args),
            _ => {
                let mut vals = Vec::with_capacity(args.len());
                for a in args {
                    vals.push(self.eval(a)?);
                }
                functions::apply(canon, &vals)
            }
        }
    }

    /// A number written for a base conversion: digits read in `base`
    /// (`hex_to_bin(FF3A)` or `hex_to_dec(10)`), or any whole-number expression.
    fn number_in_base(&mut self, e: &Expr, base: u32) -> Result<BigInt, CalcError> {
        let digits = match e {
            Expr::Num(NumLit { text, base: 10 }) if base != 10 => Some(text.clone()),
            Expr::Ident(s) if base == 16 && functions::constant(s).is_none() => Some(s.clone()),
            _ => None,
        };
        if let Some(d) = digits {
            return BigInt::parse_bytes(d.as_bytes(), base)
                .ok_or_else(|| CalcError::new(format!("'{d}' is not a number in base {base}")));
        }
        let v = self.eval(e)?;
        int_of(&v, "a base conversion")
    }

    fn bound_var(e: &Expr, what: &str) -> Result<String, CalcError> {
        match e {
            Expr::Ident(v) => Ok(v.clone()),
            _ => Err(CalcError::new(format!(
                "{what} needs a variable name as its second argument"
            ))),
        }
    }

    fn with_var<T>(
        &mut self,
        var: &str,
        value: Value,
        f: impl FnOnce(&mut Self) -> Result<T, CalcError>,
    ) -> Result<T, CalcError> {
        self.vars.push((var.to_string(), value));
        let r = f(self);
        self.vars.pop();
        r
    }

    /// `sum(f, k, a, b)` and `prod(f, k, a, b)`.
    fn series(&mut self, which: &str, args: &[Expr]) -> Result<Value, CalcError> {
        if args.len() != 4 {
            return Err(CalcError::new(format!(
                "{which} takes four arguments: {which}(expression, variable, from, to)"
            )));
        }
        let var = Self::bound_var(&args[1], which)?;
        let from = self.eval(&args[2])?;
        let to = self.eval(&args[3])?;
        let (Some(from), Some(to)) = (from.as_int(), to.as_int()) else {
            return Err(CalcError::new(format!(
                "{which} runs between whole numbers"
            )));
        };
        let (Some(from), Some(to)) = (from.to_i64(), to.to_i64()) else {
            return Err(CalcError::new(format!(
                "{which} runs between bounds of at most 9.2 × 10^18 in size"
            )));
        };
        if i128::from(to) - i128::from(from) + 1 > i128::from(MAX_TERMS) {
            return Err(CalcError::new(format!(
                "{which} over more than {} terms is too long to compute",
                group(MAX_TERMS as usize)
            )));
        }
        let mut acc = Value::Int(BigInt::from(if which == "sum" { 0 } else { 1 }));
        for k in from..=to {
            let term = self.with_var(&var, Value::Int(BigInt::from(k)), |ev| ev.eval(&args[0]))?;
            acc = arith(
                if which == "sum" {
                    BinOp::Add
                } else {
                    BinOp::Mul
                },
                &acc,
                &term,
            )?;
        }
        Ok(acc)
    }

    fn at(&mut self, f: &Expr, var: &str, x: f64) -> Result<f64, CalcError> {
        let v = self.with_var(var, Value::Real(x), |ev| ev.eval(f))?;
        let r = real(&v)?;
        if !r.is_finite() {
            return Err(CalcError::new(format!(
                "the expression is undefined at {var} = {x}"
            )));
        }
        Ok(r)
    }

    /// `deriv(f, x, a)`: central differences with Richardson extrapolation.
    fn derivative(&mut self, args: &[Expr]) -> Result<Value, CalcError> {
        if args.len() != 3 {
            return Err(CalcError::new(
                "deriv takes three arguments: deriv(expression, variable, point)",
            ));
        }
        let var = Self::bound_var(&args[1], "deriv")?;
        let a = real(&self.eval(&args[2])?)?;
        let h = 1e-3 * a.abs().max(1.0);
        let mut d = |h: f64| -> Result<f64, CalcError> {
            Ok((self.at(&args[0], &var, a + h)? - self.at(&args[0], &var, a - h)?) / (2.0 * h))
        };
        let (d1, d2, d3) = (d(h)?, d(h / 2.0)?, d(h / 4.0)?);
        let r1 = (4.0 * d2 - d1) / 3.0;
        let r2 = (4.0 * d3 - d2) / 3.0;
        let r = (16.0 * r2 - r1) / 15.0;
        let r = if r.abs() < 1e-11 { 0.0 } else { r };
        Ok(Value::Real(r))
    }

    /// `integral(f, x, a, b)`: adaptive Simpson's rule to a relative 1e-12.
    fn integral(&mut self, args: &[Expr]) -> Result<Value, CalcError> {
        if args.len() != 4 {
            return Err(CalcError::new(
                "integral takes four arguments: integral(expression, variable, from, to)",
            ));
        }
        let var = Self::bound_var(&args[1], "integral")?;
        let a = real(&self.eval(&args[2])?)?;
        let b = real(&self.eval(&args[3])?)?;
        if a == b {
            return Ok(Value::Int(BigInt::zero()));
        }
        let f = &args[0];
        let fa = self.at(f, &var, a)?;
        let fb = self.at(f, &var, b)?;
        let m = (a + b) / 2.0;
        let fm = self.at(f, &var, m)?;
        let whole = (b - a) / 6.0 * (fa + 4.0 * fm + fb);
        let r = self.simpson(
            f,
            &var,
            a,
            b,
            fa,
            fm,
            fb,
            whole,
            1e-12 * whole.abs().max(1e-12),
            48,
        )?;
        Ok(Value::Real(r))
    }

    #[allow(clippy::too_many_arguments)]
    fn simpson(
        &mut self,
        f: &Expr,
        var: &str,
        a: f64,
        b: f64,
        fa: f64,
        fm: f64,
        fb: f64,
        whole: f64,
        tol: f64,
        depth: u32,
    ) -> Result<f64, CalcError> {
        let m = (a + b) / 2.0;
        let (lm, rm) = ((a + m) / 2.0, (m + b) / 2.0);
        let flm = self.at(f, var, lm)?;
        let frm = self.at(f, var, rm)?;
        let left = (m - a) / 6.0 * (fa + 4.0 * flm + fm);
        let right = (b - m) / 6.0 * (fm + 4.0 * frm + fb);
        let delta = left + right - whole;
        if depth == 0 || delta.abs() <= 15.0 * tol {
            return Ok(left + right + delta / 15.0);
        }
        Ok(
            self.simpson(f, var, a, m, fa, flm, fm, left, tol / 2.0, depth - 1)?
                + self.simpson(f, var, m, b, fm, frm, fb, right, tol / 2.0, depth - 1)?,
        )
    }

    fn convert(&mut self, x: &Expr, target: Target) -> Result<Value, CalcError> {
        if target == Target::Dms {
            // A number is taken as decimal degrees; a marked angle is converted.
            let degrees = match x {
                Expr::Angle(inner, Unit::Degrees) => real(&self.eval(inner)?)?,
                _ if super::render::has_angle(x) => real(&self.eval(x)?)?.to_degrees(),
                _ => real(&self.eval(x)?)?,
            };
            return Ok(Value::Text(dms(degrees)));
        }
        let i = match x {
            Expr::Num(_) | Expr::Neg(_) | Expr::Ident(_) | Expr::Call(..) | Expr::Bin(..) => {
                int_of(&self.eval(x)?, "a base conversion")?
            }
            _ => int_of(&self.eval(x)?, "a base conversion")?,
        };
        Ok(match target {
            Target::Hex => Value::Text(functions::to_base(&i, 16)),
            Target::Bin => Value::Text(functions::to_base(&i, 2)),
            Target::Oct => Value::Text(functions::to_base(&i, 8)),
            _ => Value::Int(i),
        })
    }
}

fn dms(deg: f64) -> String {
    let sign = if deg < 0.0 { "-" } else { "" };
    let total = (deg.abs() * 3600.0 * 1000.0).round() / 1000.0;
    let d = (total / 3600.0).floor();
    let m = ((total - d * 3600.0) / 60.0).floor();
    let s = total - d * 3600.0 - m * 60.0;
    format!(
        "{sign}{}°{}'{}\"",
        d as i64,
        m as i64,
        super::value::show_real((s * 1000.0).round() / 1000.0)
    )
}

/// `hex_to_bin`, `dec2hex`, `binary_to_decimal`, …: the two bases named.
fn base_pair(name: &str) -> Option<(u32, u32)> {
    let lower = name.to_ascii_lowercase();
    let (from, to) = lower
        .split_once("_to_")
        .or_else(|| lower.split_once("to"))
        .or_else(|| lower.split_once('2'))?;
    let base = |s: &str| match s.trim_matches('_') {
        "hex" | "hexadecimal" => Some(16),
        "bin" | "binary" => Some(2),
        "oct" | "octal" => Some(8),
        "dec" | "decimal" => Some(10),
        _ => None,
    };
    Some((base(from)?, base(to)?))
}

fn number(n: &NumLit) -> Result<Value, CalcError> {
    if n.base != 10 {
        return BigInt::parse_bytes(n.text.as_bytes(), n.base)
            .map(Value::Int)
            .ok_or_else(|| {
                CalcError::new(format!("'{}' is not a number in base {}", n.text, n.base))
            });
    }
    if n.text.chars().all(|c| c.is_ascii_digit()) {
        return Ok(Value::Int(n.text.parse::<BigInt>().map_err(|_| {
            CalcError::new(format!("'{}' is not a number", n.text))
        })?));
    }
    n.text
        .parse::<f64>()
        .map(Value::Real)
        .map_err(|_| CalcError::new(format!("'{}' is not a number", n.text)))
}

fn real(v: &Value) -> Result<f64, CalcError> {
    match v {
        Value::Text(t) => Err(CalcError::new(format!(
            "'{t}' is a result to read, not a number to calculate with"
        ))),
        _ => v
            .as_f64()
            .ok_or_else(|| CalcError::new("a number is too large to calculate with")),
    }
}

fn int_of(v: &Value, what: &str) -> Result<BigInt, CalcError> {
    v.as_int().ok_or_else(|| {
        CalcError::new(format!(
            "{what} needs a whole number, not {}",
            super::value::show(v)
        ))
    })
}

fn digits_of_power(base: &BigInt, exp: u64) -> f64 {
    let b = base.abs();
    if b <= BigInt::from(1) {
        return 1.0;
    }
    // log10 of the base, from its value where it fits a float, else from its length.
    let log = match b.to_f64() {
        Some(f) if f.is_finite() => f.log10(),
        _ => (b.bits() - 1) as f64 * std::f64::consts::LOG10_2,
    };
    (exp as f64 * log).floor() + 1.0
}

pub fn arith(op: BinOp, x: &Value, y: &Value) -> Result<Value, CalcError> {
    use BinOp::*;
    if let (Value::Int(a), Value::Int(b)) = (x, y) {
        match op {
            Add => return Ok(Value::Int(a + b)),
            Sub => return Ok(Value::Int(a - b)),
            Mul => {
                let bits = a.bits() + b.bits();
                if bits > MAX_INT_DIGITS * 10 / 3 + 4 {
                    return Err(CalcError::new(format!(
                        "the result has about {} digits, beyond the {} this calculator works with",
                        group((bits as f64 * std::f64::consts::LOG10_2) as usize),
                        group(MAX_INT_DIGITS as usize)
                    )));
                }
                return Ok(Value::Int(a * b));
            }
            Div => {
                if b.is_zero() {
                    return Err(CalcError::new("division by zero is undefined"));
                }
                if (a % b).is_zero() {
                    return Ok(Value::Int(a / b));
                }
            }
            Pow if !b.is_negative() => {
                let one = BigInt::from(1);
                if a.is_zero() {
                    return Ok(Value::Int(if b.is_zero() { one } else { BigInt::zero() }));
                }
                if *a == one {
                    return Ok(Value::Int(one));
                }
                if *a == -one.clone() {
                    let even = (b % 2u32).is_zero();
                    return Ok(Value::Int(if even { one } else { -one }));
                }
                let e = b.to_u64().unwrap_or(u64::MAX);
                let digits = digits_of_power(a, e);
                if digits <= MAX_INT_DIGITS as f64 && e <= u64::from(u32::MAX) {
                    return Ok(Value::Int(a.pow(e as u32)));
                }
                return Err(CalcError::new(format!(
                    "the result has about {} digits, beyond the {} this calculator works with",
                    group(digits as usize),
                    group(MAX_INT_DIGITS as usize)
                )));
            }
            Mod => return functions::modulo(x, y),
            Choose => return functions::ncr(a, b),
            And | Or | Xor | Xnor | Nand => {
                let name = format!("{op:?}").to_lowercase();
                return functions::bitwise(&name, a, b);
            }
            Shl | Shr => {
                let s = b.to_i64().unwrap_or(i64::MAX);
                if s.unsigned_abs() > MAX_INT_DIGITS * 3 {
                    return Err(CalcError::new("a shift that large is too large to compute"));
                }
                let left = (op == Shl) == (s >= 0);
                let s = s.unsigned_abs() as usize;
                return Ok(Value::Int(if left { a << s } else { a >> s }));
            }
            _ => {}
        }
    }
    match op {
        Mod => return functions::modulo(x, y),
        Choose => {
            return match (x.as_int(), y.as_int()) {
                (Some(a), Some(b)) => functions::ncr(&a, &b),
                _ => Err(CalcError::new("choose needs whole numbers")),
            };
        }
        And | Or | Xor | Xnor | Nand | Shl | Shr => {
            return match (x.as_int(), y.as_int()) {
                (Some(a), Some(b)) => arith(op, &Value::Int(a), &Value::Int(b)),
                _ => Err(CalcError::new("bitwise operations need whole numbers")),
            };
        }
        _ => {}
    }
    let (a, b) = (real(x)?, real(y)?);
    let r = match op {
        Add => a + b,
        Sub => a - b,
        Mul => a * b,
        Div => {
            if b == 0.0 {
                return Err(CalcError::new("division by zero is undefined"));
            }
            a / b
        }
        Pow => {
            if a == 0.0 && b < 0.0 {
                return Err(CalcError::new("zero to a negative power is undefined"));
            }
            if a < 0.0 && b.fract() != 0.0 {
                // p/q with q odd: (-8)^(1/3) is -2 and (-8)^(2/3) is 4.
                let Some((p, q)) = (1..100i64).step_by(2).find_map(|q| {
                    let p = (b * q as f64).round();
                    ((p / q as f64 - b).abs() < 1e-12).then_some((p as i64, q))
                }) else {
                    return Err(CalcError::new(
                        "a negative number to a fractional power is not a real number",
                    ));
                };
                let root = functions::nth_root(a, q as f64)?;
                root.abs()
                    .powi(p.unsigned_abs() as i32)
                    .powf(p.signum() as f64)
                    * if p % 2 == 0 { 1.0 } else { root.signum() }
            } else if b.fract() != 0.0 && b != 0.0 && (1.0 / b - (1.0 / b).round()).abs() < 1e-9 {
                // x^(1/n) is the nth root, exact where the root is.
                functions::nth_root(a, (1.0 / b).round())?
            } else {
                a.powf(b)
            }
        }
        _ => unreachable!(),
    };
    if r.is_nan() {
        return Err(CalcError::new("the result is undefined"));
    }
    if r.is_infinite() {
        return Err(CalcError::new(
            "the result is too large (beyond about 1.8 × 10^308)",
        ));
    }
    Ok(Value::Real(r))
}
