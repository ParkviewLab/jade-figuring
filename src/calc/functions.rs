// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The function table: every accepted name, its canonical form, and the
//! numeric functions themselves.

use std::f64::consts::PI;

use num_bigint::BigInt;
use num_integer::Integer;
use num_traits::{One, Signed, ToPrimitive, Zero};

use super::CalcError;
use super::value::{MAX_INT_DIGITS, Value};

/// Canonical names, each with the other names accepted for it.
const NAMES: &[(&str, &[&str])] = &[
    ("sqrt", &["squareroot"]),
    ("cbrt", &["cuberoot"]),
    ("root", &["nthroot", "rt"]),
    ("abs", &["absolute", "fabs"]),
    ("round", &["rnd"]),
    ("floor", &["int"]),
    ("ceil", &["ceiling"]),
    ("ipart", &["trunc", "truncate", "intpart", "integerpart"]),
    ("fpart", &["frac", "fracpart", "fractionalpart"]),
    ("min", &[]),
    ("max", &[]),
    ("mod", &["modulo", "rem"]),
    ("gcd", &["gcf", "hcf"]),
    ("lcm", &[]),
    (
        "factors",
        &[
            "factor",
            "primefactors",
            "factorize",
            "factorise",
            "pfactor",
        ],
    ),
    (
        "ncr",
        &[
            "comb",
            "combinations",
            "combination",
            "choose",
            "binomial",
            "binom",
            "c",
        ],
    ),
    ("npr", &["perm", "permutations", "permutation", "p"]),
    ("factorial", &["fact"]),
    ("exp", &[]),
    ("ln", &["loge"]),
    ("log", &[]),
    ("log10", &["lg", "logten"]),
    ("log2", &["lb", "ld"]),
    ("sin", &["sine"]),
    ("cos", &["cosine"]),
    ("tan", &["tangent"]),
    ("sec", &[]),
    ("csc", &["cosec"]),
    ("cot", &["cotan"]),
    ("asin", &["arcsin", "sininv", "invsin"]),
    ("acos", &["arccos", "cosinv", "invcos"]),
    ("atan", &["arctan", "taninv", "invtan"]),
    ("atan2", &["arctan2"]),
    ("sinh", &[]),
    ("cosh", &[]),
    ("tanh", &[]),
    ("asinh", &["arcsinh", "arsinh"]),
    ("acosh", &["arccosh", "arcosh"]),
    ("atanh", &["arctanh", "artanh"]),
    (
        "polar",
        &["topolar", "rtop", "r2p", "rectangulartopolar", "cart2pol"],
    ),
    (
        "rect",
        &["torect", "ptor", "p2r", "polartorectangular", "pol2cart"],
    ),
    ("todeg", &["degrees", "rad2deg", "radtodeg"]),
    ("torad", &["radians", "deg2rad", "degtorad"]),
    ("hex", &["tohex"]),
    ("bin", &["tobin", "binary"]),
    ("oct", &["tooct", "octal"]),
    ("dec", &["todec", "decimal"]),
    ("dms", &["todms"]),
    ("twos", &["twoscomplement", "complement2"]),
    ("and", &["bitand", "band"]),
    ("or", &["bitor", "bor"]),
    ("xor", &["bitxor", "bxor"]),
    ("xnor", &[]),
    ("nand", &[]),
    ("not", &["bitnot", "bnot"]),
    (
        "deriv",
        &["derivative", "nderiv", "diff", "differentiate", "d"],
    ),
    ("integral", &["integrate", "nint", "fnint", "defint"]),
    ("sum", &["sigma", "summation"]),
    ("prod", &["product"]),
];

/// The canonical name for `name`, if it names a function.
pub fn canonical(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    // `C` and `P` alone are combinations only when called: `C(52, 5)`.
    for (canon, aliases) in NAMES {
        if *canon == lower || aliases.contains(&lower.as_str()) {
            return Some(canon);
        }
    }
    None
}

/// Whether `name` is a function that may be written without brackets
/// (`ln 50`); single letters never are.
pub fn is_function(name: &str) -> bool {
    name.len() > 1 && canonical(name).is_some() && !matches!(name, "deg" | "rad")
}

/// Every accepted name, for suggestions.
pub fn all_names() -> impl Iterator<Item = &'static str> {
    NAMES
        .iter()
        .flat_map(|(c, a)| std::iter::once(*c).chain(a.iter().copied()))
}

pub fn constant(name: &str) -> Option<f64> {
    Some(match name {
        "pi" | "PI" | "Pi" => PI,
        "e" | "E" => std::f64::consts::E,
        "tau" => std::f64::consts::TAU,
        "phi" => std::f64::consts::GOLDEN_RATIO,
        // Physical constants, at the CODATA 2018 values engineering calculators use.
        "c" => 299_792_458.0,
        "g" => 9.80665,
        "h" => 6.626_070_15e-34,
        "NA" | "N_A" => 6.022_140_76e23,
        "R" => 8.314_462_618,
        "me" | "m_e" => 9.109_383_701_5e-31,
        "mp" | "m_p" => 1.672_621_923_69e-27,
        "mn" | "m_n" => 1.674_927_498_04e-27,
        "mmu" | "m_mu" => 1.883_531_627e-28,
        "G" => 6.6743e-11,
        "F" => 96_485.332_12,
        "a0" | "a_0" => 5.291_772_109_03e-11,
        "re" | "r_e" => 2.817_940_326_2e-15,
        "k" | "kB" | "k_B" => 1.380_649e-23,
        "qe" | "q_e" => 1.602_176_634e-19,
        "u" | "amu" => 1.660_539_066_60e-27,
        "atm" => 101_325.0,
        "eps0" | "epsilon0" | "epsilon_0" => 8.854_187_812_8e-12,
        "mu0" | "mu_0" => 1.256_637_062_12e-6,
        "ke" | "k_e" => 8.987_551_792_3e9,
        _ => return None,
    })
}

/// The nearest accepted name, for "unknown function 'sine'; use sin".
pub fn suggest(name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    if let Some(c) = canonical(&lower) {
        return Some(display(c).into());
    }
    if lower.chars().count() < 2 {
        return None;
    }
    let reach = if lower.chars().count() <= 3 { 1 } else { 2 };
    let mut best: Option<(usize, &str)> = None;
    for cand in all_names().filter(|n| n.len() > 1) {
        let d = distance(&lower, cand);
        if d <= reach && best.is_none_or(|(bd, _)| d < bd) {
            best = Some((d, cand));
        }
    }
    best.map(|(_, c)| display(canonical(c).unwrap_or(c)).to_string())
}

/// How a canonical name is written to the reader.
pub fn display(canon: &str) -> &str {
    match canon {
        "ncr" => "nCr",
        "npr" => "nPr",
        other => other,
    }
}

fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for i in 1..=a.len() {
        let mut cur = vec![i; b.len() + 1];
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        prev = cur;
    }
    prev[b.len()]
}

fn real(v: &Value, f: &str) -> Result<f64, CalcError> {
    v.as_f64()
        .ok_or_else(|| CalcError::new(format!("{f} needs a number")))
}

fn whole(v: &Value, f: &str) -> Result<BigInt, CalcError> {
    v.as_int()
        .ok_or_else(|| CalcError::new(format!("{f} needs whole numbers")))
}

fn arity(name: &str, args: &[Value], n: usize) -> Result<(), CalcError> {
    if args.len() == n {
        Ok(())
    } else {
        Err(CalcError::new(format!(
            "{name} takes {n} argument{}, not {}",
            if n == 1 { "" } else { "s" },
            args.len()
        )))
    }
}

/// A trigonometric value within rounding of zero is zero (sin(pi) is 0), but
/// not where the argument itself is that small (sin(1e-20) is 1e-20).
fn snap(value: f64, argument: f64) -> f64 {
    if value.abs() < 1e-15 && argument.abs() > 1e-6 {
        0.0
    } else {
        value
    }
}

/// A coordinate within rounding of zero, relative to the radius, is zero.
fn near_zero(v: f64, r: f64, t: f64) -> f64 {
    if v.abs() < 1e-15 * r.abs().max(1.0) && t.abs() > 1e-6 {
        0.0
    } else {
        v
    }
}

fn finite(x: f64, what: &str) -> Result<Value, CalcError> {
    if x.is_nan() {
        Err(CalcError::new(format!("{what} is undefined here")))
    } else if x.is_infinite() {
        Err(CalcError::new(format!(
            "the result of {what} is too large (beyond about 1.8 × 10^308)"
        )))
    } else {
        Ok(Value::Real(x))
    }
}

pub fn factorial(n: &BigInt) -> Result<Value, CalcError> {
    if n.is_negative() {
        return Err(CalcError::new(
            "the factorial of a negative number is undefined",
        ));
    }
    let n = n.to_u64().unwrap_or(u64::MAX);
    let digits = lgamma_digits(n as f64 + 1.0);
    if digits > MAX_INT_DIGITS as f64 {
        return Err(CalcError::new(format!(
            "{n}! has about {} digits, beyond the {} this calculator works with",
            super::value::group(digits as usize),
            super::value::group(MAX_INT_DIGITS as usize)
        )));
    }
    let mut acc = BigInt::one();
    for k in 2..=n {
        acc *= k;
    }
    Ok(Value::Int(acc))
}

/// Digits of Γ(x) = (x-1)!, by Stirling's series.
fn lgamma_digits(x: f64) -> f64 {
    if x < 3.0 {
        return 1.0;
    }
    let n = x - 1.0;
    let ln = n * n.ln() - n + 0.5 * (2.0 * PI * n).ln() + 1.0 / (12.0 * n);
    ln / std::f64::consts::LN_10 + 1.0
}

pub fn ncr(n: &BigInt, k: &BigInt) -> Result<Value, CalcError> {
    if n.is_negative() || k.is_negative() {
        return Err(CalcError::new(
            "combinations need whole numbers of zero or more",
        ));
    }
    if k > n {
        return Ok(Value::Int(BigInt::zero()));
    }
    let k = std::cmp::min(k.clone(), n - k);
    let kk = k.to_u64().unwrap_or(u64::MAX);
    if kk > 100_000 {
        return Err(CalcError::new(
            "combinations of more than 100,000 items are too large to compute",
        ));
    }
    let mut acc = BigInt::one();
    for i in 0..kk {
        acc = acc * (n - i) / (i + 1);
        if acc.bits() > MAX_INT_DIGITS * 10 / 3 {
            return Err(CalcError::new(format!(
                "the number of combinations has more than {} digits, beyond what this calculator works with",
                super::value::group(MAX_INT_DIGITS as usize)
            )));
        }
    }
    Ok(Value::Int(acc))
}

pub fn npr(n: &BigInt, k: &BigInt) -> Result<Value, CalcError> {
    if n.is_negative() || k.is_negative() {
        return Err(CalcError::new(
            "permutations need whole numbers of zero or more",
        ));
    }
    if k > n {
        return Ok(Value::Int(BigInt::zero()));
    }
    let kk = k.to_u64().unwrap_or(u64::MAX);
    if kk > 100_000 {
        return Err(CalcError::new(
            "permutations of more than 100,000 items are too large to compute",
        ));
    }
    let mut acc = BigInt::one();
    for i in 0..kk {
        acc *= n - i;
        if acc.bits() > MAX_INT_DIGITS * 10 / 3 {
            return Err(CalcError::new(format!(
                "the number of permutations has more than {} digits, beyond what this calculator works with",
                super::value::group(MAX_INT_DIGITS as usize)
            )));
        }
    }
    Ok(Value::Int(acc))
}

/// Rounds to `places` decimal places, half away from zero, on the decimal
/// digits shown (so 2.345 rounds to 2.35, as on a calculator).
pub fn round_places(x: f64, places: i32) -> f64 {
    if places > 15 {
        return x;
    }
    if places < -308 {
        return 0.0;
    }
    let s = format!("{:.15e}", x);
    let v: f64 = s.parse().unwrap_or(x);
    let scale = 10f64.powi(places);
    let scaled: f64 = format!("{:.12e}", v * scale).parse().unwrap_or(v * scale);
    let r = scaled.abs().round() * scaled.signum();
    r / scale
}

/// The real `n`th root, negative for an odd root of a negative number.
pub fn nth_root(x: f64, n: f64) -> Result<f64, CalcError> {
    if n == 0.0 {
        return Err(CalcError::new("the 0th root is undefined"));
    }
    if x < 0.0 {
        let odd = n.fract() == 0.0 && (n as i64) % 2 != 0;
        if !odd {
            return Err(CalcError::new(
                "an even root of a negative number is not a real number",
            ));
        }
        return Ok(-(-x).powf(1.0 / n));
    }
    let r = x.powf(1.0 / n);
    // Exact roots come out exact: 9261^(1/3) is 21, not 20.999999999999996.
    let near = r.round();
    if (near.powf(n) - x).abs() <= x.abs() * 1e-14 {
        return Ok(near);
    }
    Ok(r)
}

pub fn prime_factors(n: &BigInt) -> Result<Value, CalcError> {
    let Some(mut m) = n.to_u64() else {
        return Err(CalcError::new(
            "factors handles whole numbers from 2 up to 10^14",
        ));
    };
    if !(2..=100_000_000_000_000).contains(&m) {
        return Err(CalcError::new(
            "factors handles whole numbers from 2 up to 10^14",
        ));
    }
    let mut parts: Vec<(u64, u32)> = Vec::new();
    let mut p = 2u64;
    while p * p <= m {
        let mut k = 0;
        while m % p == 0 {
            m /= p;
            k += 1;
        }
        if k > 0 {
            parts.push((p, k));
        }
        p += if p == 2 { 1 } else { 2 };
    }
    if m > 1 {
        parts.push((m, 1));
    }
    let text = parts
        .iter()
        .map(|(p, k)| {
            if *k == 1 {
                p.to_string()
            } else {
                format!("{p}^{k}")
            }
        })
        .collect::<Vec<_>>()
        .join(" × ");
    Ok(Value::Text(text))
}

pub fn to_base(i: &BigInt, base: u32) -> String {
    let prefix = match base {
        16 => "0x",
        2 => "0b",
        8 => "0o",
        _ => "",
    };
    let digits = i.abs().to_str_radix(base).to_uppercase();
    let sign = if i.is_negative() { "-" } else { "" };
    format!("{sign}{prefix}{digits}")
}

pub fn twos(i: &BigInt, bits: u32) -> Result<Value, CalcError> {
    if ![8, 16, 32, 64].contains(&bits) {
        return Err(CalcError::new(
            "two's complement takes a width of 8, 16, 32 or 64 bits",
        ));
    }
    let modulus = BigInt::one() << bits;
    let min = -(BigInt::one() << (bits - 1));
    if *i < min || *i >= modulus {
        return Err(CalcError::new(format!("{i} does not fit in {bits} bits")));
    }
    let v = i.mod_floor(&modulus);
    let b = format!("{:0>width$}", v.to_str_radix(2), width = bits as usize);
    let h = format!(
        "{:0>width$}",
        v.to_str_radix(16).to_uppercase(),
        width = (bits / 4) as usize
    );
    Ok(Value::Text(format!("0b{b} (0x{h}, {bits} bits)")))
}

pub fn bitnot(i: &BigInt, bits: Option<u32>) -> Result<Value, CalcError> {
    match bits {
        None => Ok(Value::Int(-i - 1)),
        Some(b) => {
            if b == 0 || b > 64 {
                return Err(CalcError::new("not takes a width of 1 to 64 bits"));
            }
            let mask = (BigInt::one() << b) - 1;
            Ok(Value::Int((!i) & mask))
        }
    }
}

/// Applies a numeric function to evaluated arguments.
pub fn apply(name: &str, args: &[Value]) -> Result<Value, CalcError> {
    let one = |f: fn(f64) -> f64, what: &str| -> Result<Value, CalcError> {
        arity(name, args, 1)?;
        finite(f(real(&args[0], name)?), what)
    };
    match name {
        "sqrt" => {
            arity(name, args, 1)?;
            if let Some(i) = args[0].as_int()
                && !i.is_negative()
            {
                let r = i.sqrt();
                if &r * &r == i {
                    return Ok(Value::Int(r));
                }
            }
            let x = real(&args[0], name)?;
            if x < 0.0 {
                return Err(CalcError::new(
                    "the square root of a negative number is not a real number",
                ));
            }
            finite(x.sqrt(), "sqrt")
        }
        "cbrt" => {
            arity(name, args, 1)?;
            finite(nth_root(real(&args[0], name)?, 3.0)?, "cbrt")
        }
        "root" => {
            arity(name, args, 2)?;
            finite(
                nth_root(real(&args[0], name)?, real(&args[1], name)?)?,
                "root",
            )
        }
        "abs" => {
            arity(name, args, 1)?;
            Ok(match &args[0] {
                Value::Int(i) => Value::Int(i.abs()),
                v => Value::Real(real(v, name)?.abs()),
            })
        }
        "round" => {
            if args.len() == 1 {
                if let Value::Int(_) = args[0] {
                    return Ok(args[0].clone());
                }
                let x = real(&args[0], name)?;
                return Ok(int_or_real(round_places(x, 0)));
            }
            arity(name, args, 2)?;
            let places = real(&args[1], name)? as i32;
            let x = real(&args[0], name)?;
            let r = round_places(x, places);
            Ok(if places <= 0 {
                int_or_real(r)
            } else {
                Value::Real(r)
            })
        }
        "floor" | "ceil" | "ipart" => {
            arity(name, args, 1)?;
            if let Value::Int(_) = args[0] {
                return Ok(args[0].clone());
            }
            let x = real(&args[0], name)?;
            Ok(int_or_real(match name {
                "floor" => x.floor(),
                "ceil" => x.ceil(),
                _ => x.trunc(),
            }))
        }
        "fpart" => {
            arity(name, args, 1)?;
            let x = real(&args[0], name)?;
            Ok(Value::Real(x.fract()))
        }
        "min" | "max" => {
            if args.is_empty() {
                return Err(CalcError::new(format!("{name} needs at least one number")));
            }
            let mut best = &args[0];
            for a in &args[1..] {
                let (x, y) = (real(a, name)?, real(best, name)?);
                if (name == "min" && x < y) || (name == "max" && x > y) {
                    best = a;
                }
            }
            Ok(best.clone())
        }
        "mod" => {
            arity(name, args, 2)?;
            modulo(&args[0], &args[1])
        }
        "gcd" | "lcm" => {
            if args.len() < 2 {
                return Err(CalcError::new(format!(
                    "{name} needs at least two whole numbers"
                )));
            }
            let mut acc = whole(&args[0], name)?;
            for a in &args[1..] {
                let b = whole(a, name)?;
                acc = if name == "gcd" {
                    acc.gcd(&b)
                } else {
                    acc.lcm(&b)
                };
            }
            Ok(Value::Int(acc))
        }
        "factors" => {
            arity(name, args, 1)?;
            prime_factors(&whole(&args[0], name)?)
        }
        "ncr" => {
            arity(name, args, 2)?;
            ncr(&whole(&args[0], name)?, &whole(&args[1], name)?)
        }
        "npr" => {
            arity(name, args, 2)?;
            npr(&whole(&args[0], name)?, &whole(&args[1], name)?)
        }
        "factorial" => {
            arity(name, args, 1)?;
            match args[0].as_int() {
                Some(n) => factorial(&n),
                None => Err(CalcError::new("the factorial needs a whole number")),
            }
        }
        "exp" => one(f64::exp, "exp"),
        "ln" | "log" | "log10" | "log2" => {
            let (x, base) = match (name, args.len()) {
                ("log", 2) => (real(&args[0], name)?, Some(real(&args[1], name)?)),
                (_, 1) => (real(&args[0], name)?, None),
                _ => {
                    return Err(CalcError::new(format!(
                        "{name} takes one argument{}",
                        if name == "log" {
                            ", or two for log(x, base)"
                        } else {
                            ""
                        }
                    )));
                }
            };
            if x == 0.0 {
                return Err(CalcError::new("the logarithm of zero is undefined"));
            }
            if x < 0.0 {
                return Err(CalcError::new(
                    "the logarithm of a negative number is not a real number",
                ));
            }
            let r = match (name, base) {
                ("ln", _) => x.ln(),
                ("log2", _) => x.log2(),
                (_, Some(b)) => {
                    if b <= 0.0 || b == 1.0 {
                        return Err(CalcError::new(
                            "a logarithm's base must be positive and not 1",
                        ));
                    }
                    exact_log(x, b)
                }
                _ => x.log10(),
            };
            finite(r, name)
        }
        "sin" | "cos" | "tan" | "sec" | "csc" | "cot" => {
            arity(name, args, 1)?;
            let x = real(&args[0], name)?;
            let (s, c) = (snap(x.sin(), x), snap(x.cos(), x));
            let r = match name {
                "sin" => s,
                "cos" => c,
                "tan" => ratio(s, c, "tan")?,
                "sec" => ratio(1.0, c, "sec")?,
                "csc" => ratio(1.0, s, "csc")?,
                _ => ratio(c, s, "cot")?,
            };
            Ok(Value::Real(if r.abs() < 1e-15 && x.abs() > 1e-6 {
                0.0
            } else {
                r
            }))
        }
        "asin" | "acos" => {
            arity(name, args, 1)?;
            let x = real(&args[0], name)?;
            if !(-1.0..=1.0).contains(&x) {
                return Err(CalcError::new(format!(
                    "{name} needs a value between -1 and 1"
                )));
            }
            Ok(Value::Real(if name == "asin" {
                x.asin()
            } else {
                x.acos()
            }))
        }
        "atan" => one(f64::atan, "atan"),
        "atan2" => {
            arity(name, args, 2)?;
            Ok(Value::Real(
                real(&args[0], name)?.atan2(real(&args[1], name)?),
            ))
        }
        "sinh" => one(f64::sinh, "sinh"),
        "cosh" => one(f64::cosh, "cosh"),
        "tanh" => one(f64::tanh, "tanh"),
        "asinh" => one(f64::asinh, "asinh"),
        "acosh" => {
            arity(name, args, 1)?;
            let x = real(&args[0], name)?;
            if x < 1.0 {
                return Err(CalcError::new("acosh needs a value of 1 or more"));
            }
            Ok(Value::Real(x.acosh()))
        }
        "atanh" => {
            arity(name, args, 1)?;
            let x = real(&args[0], name)?;
            if x <= -1.0 || x >= 1.0 {
                return Err(CalcError::new(
                    "atanh needs a value strictly between -1 and 1",
                ));
            }
            Ok(Value::Real(x.atanh()))
        }
        "polar" => {
            arity(name, args, 2)?;
            let (x, y) = (real(&args[0], name)?, real(&args[1], name)?);
            let r = x.hypot(y);
            let t = y.atan2(x);
            Ok(Value::Text(format!(
                "r = {}, θ = {} ({}°)",
                super::value::show_real(r),
                super::value::show_real(t),
                super::value::show_real(t.to_degrees())
            )))
        }
        "rect" => {
            arity(name, args, 2)?;
            let (r, t) = (real(&args[0], name)?, real(&args[1], name)?);
            Ok(Value::Text(format!(
                "x = {}, y = {}",
                super::value::show_real(near_zero(r * t.cos(), r, t)),
                super::value::show_real(near_zero(r * t.sin(), r, t))
            )))
        }
        "todeg" => one(f64::to_degrees, "todeg"),
        "torad" => one(f64::to_radians, "torad"),
        "hex" | "bin" | "oct" | "dec" => {
            arity(name, args, 1)?;
            let i = whole(&args[0], "a base conversion")?;
            Ok(match name {
                "hex" => Value::Text(to_base(&i, 16)),
                "bin" => Value::Text(to_base(&i, 2)),
                "oct" => Value::Text(to_base(&i, 8)),
                _ => Value::Int(i),
            })
        }
        "twos" => {
            arity(name, args, 2)?;
            let bits = whole(&args[1], name)?.to_u32().unwrap_or(0);
            twos(&whole(&args[0], name)?, bits)
        }
        "and" | "or" | "xor" | "xnor" | "nand" => {
            arity(name, args, 2)?;
            bitwise(name, &whole(&args[0], name)?, &whole(&args[1], name)?)
        }
        "not" => match args.len() {
            1 => bitnot(&whole(&args[0], name)?, None),
            2 => bitnot(
                &whole(&args[0], name)?,
                Some(whole(&args[1], name)?.to_u32().unwrap_or(0)),
            ),
            _ => Err(CalcError::new(
                "not takes a number, and optionally a width in bits",
            )),
        },
        other => Err(CalcError::new(format!("{other} cannot be used here"))),
    }
}

/// log(8, 2) is 3, not 2.9999999999999996.
fn exact_log(x: f64, b: f64) -> f64 {
    let r = x.ln() / b.ln();
    let near = r.round();
    if (b.powf(near) - x).abs() <= x.abs() * 1e-14 {
        near
    } else {
        r
    }
}

fn ratio(a: f64, b: f64, what: &str) -> Result<f64, CalcError> {
    if b == 0.0 {
        Err(CalcError::new(format!("{what} is undefined at this angle")))
    } else {
        Ok(a / b)
    }
}

fn int_or_real(x: f64) -> Value {
    if x.is_finite() && x.abs() < 9.0e15 {
        Value::Int(BigInt::from(x as i64))
    } else {
        Value::Real(x)
    }
}

pub fn modulo(a: &Value, b: &Value) -> Result<Value, CalcError> {
    if b.is_zero() {
        return Err(CalcError::new(
            "the remainder on division by zero is undefined",
        ));
    }
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Ok(Value::Int(x.mod_floor(y))),
        _ => {
            let (x, y) = (real(a, "mod")?, real(b, "mod")?);
            Ok(Value::Real(x - y * (x / y).floor()))
        }
    }
}

pub fn bitwise(name: &str, a: &BigInt, b: &BigInt) -> Result<Value, CalcError> {
    Ok(Value::Int(match name {
        "and" => a & b,
        "or" => a | b,
        "xor" => a ^ b,
        "xnor" => !(a ^ b),
        _ => !(a & b),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suggestions() {
        assert_eq!(suggest("sine").as_deref(), Some("sin"));
        assert_eq!(suggest("sqroot").as_deref(), Some("sqrt"));
        assert_eq!(suggest("arcsin").as_deref(), Some("asin"));
        assert_eq!(suggest("zzzzzz"), None);
    }

    #[test]
    fn rounding_is_decimal() {
        assert_eq!(round_places(2.345, 2), 2.35);
        assert_eq!(round_places(2.5, 0), 3.0);
        assert_eq!(round_places(-2.5, 0), -3.0);
        assert_eq!(round_places(1.005, 2), 1.01);
    }

    #[test]
    fn exact_roots() {
        assert_eq!(nth_root(9261.0, 3.0).unwrap(), 21.0);
        assert_eq!(nth_root(-8.0, 3.0).unwrap(), -2.0);
        assert!(nth_root(-8.0, 2.0).is_err());
    }
}
