// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Values and their display: exact whole numbers, decimals shown to 10
//! significant digits, and text results (conversions, factorisations).

use num_bigint::BigInt;
use num_traits::{Signed, ToPrimitive, Zero};

/// Whole numbers are shown in full up to this many digits.
pub const MAX_SHOWN_DIGITS: usize = 300;
/// No whole number beyond this many digits is computed exactly.
pub const MAX_INT_DIGITS: u64 = 10_000;
/// Significant digits shown for a decimal result.
pub const SIG_DIGITS: usize = 10;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(BigInt),
    Real(f64),
    Text(String),
}

impl Value {
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(i) => i.to_f64(),
            Value::Real(r) => Some(*r),
            Value::Text(_) => None,
        }
    }

    /// The value as a whole number, if it is one (a decimal counts when it is integral).
    pub fn as_int(&self) -> Option<BigInt> {
        match self {
            Value::Int(i) => Some(i.clone()),
            Value::Real(r) if r.is_finite() && r.fract() == 0.0 && r.abs() < 9.0e15 => {
                Some(BigInt::from(*r as i64))
            }
            _ => None,
        }
    }

    pub fn is_zero(&self) -> bool {
        match self {
            Value::Int(i) => i.is_zero(),
            Value::Real(r) => *r == 0.0,
            Value::Text(_) => false,
        }
    }
}

/// A whole number for display: in full up to 300 digits, else approximately
/// with its digit count.
pub fn show_int(i: &BigInt) -> String {
    let s = i.abs().to_string();
    let sign = if i.is_negative() { "-" } else { "" };
    if s.len() <= MAX_SHOWN_DIGITS {
        return format!("{sign}{s}");
    }
    let mut exp = s.len() - 1;
    let m: f64 = format!("{}.{}", &s[..1], &s[1..16]).parse().unwrap_or(0.0);
    let mut mant = format!("{:.*}", SIG_DIGITS - 1, m);
    if mant.starts_with("10") {
        mant = "1".into();
        exp += 1;
    }
    format!(
        "{sign}about {} × 10^{} (a whole number of {} digits)",
        trim_zeros(&mant),
        exp,
        group(s.len())
    )
}

/// Digit groups for counts in messages: 369693100 → 369,693,100.
pub fn group(n: usize) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (k, c) in s.chars().enumerate() {
        if k > 0 && (s.len() - k).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

fn trim_zeros(s: &str) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s.to_string()
    }
}

/// A decimal to 10 significant digits, plain between 10^-6 and 10^15,
/// otherwise as `m × 10^n`.
pub fn show_real(x: f64) -> String {
    if x == 0.0 {
        return "0".into();
    }
    if !x.is_finite() {
        return "undefined".into();
    }
    let sci = format!("{:.*e}", SIG_DIGITS - 1, x);
    let (mant, exp) = sci.split_once('e').unwrap();
    let exp: i32 = exp.parse().unwrap();
    if (-6..15).contains(&exp) {
        // Rebuild the rounded value in plain notation from the rounded digits.
        let neg = mant.starts_with('-');
        let digits: String = mant.chars().filter(|c| c.is_ascii_digit()).collect();
        let plain = if exp >= 0 {
            let int_len = exp as usize + 1;
            if digits.len() > int_len {
                format!("{}.{}", &digits[..int_len], &digits[int_len..])
            } else {
                format!("{digits}{}", "0".repeat(int_len - digits.len()))
            }
        } else {
            format!("0.{}{}", "0".repeat((-exp - 1) as usize), digits)
        };
        let plain = trim_zeros(&plain);
        if neg { format!("-{plain}") } else { plain }
    } else {
        format!("{} × 10^{}", trim_zeros(mant), exp)
    }
}

pub fn show(v: &Value) -> String {
    match v {
        Value::Int(i) => show_int(i),
        Value::Real(r) => show_real(*r),
        Value::Text(t) => t.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ten_significant_digits() {
        assert_eq!(show_real(0.1 + 0.2), "0.3");
        assert_eq!(show_real(0.49999999999999994), "0.5");
        assert_eq!(show_real(2.0f64.sqrt()), "1.414213562");
        assert_eq!(show_real(409.5), "409.5");
        assert_eq!(show_real(-0.000123456789012), "-0.000123456789");
        assert_eq!(show_real(1.2676506002282294e30), "1.2676506 × 10^30");
        assert_eq!(show_real(6.62607015e-34), "6.62607015 × 10^-34");
        assert_eq!(show_real(123456789012.0), "123456789000");
    }

    #[test]
    fn large_whole_numbers() {
        let big: BigInt = BigInt::from(2).pow(100);
        assert_eq!(show_int(&big), "1267650600228229401496703205376");
        let huge: BigInt = BigInt::from(10).pow(456) * 3;
        assert_eq!(
            show_int(&huge),
            "about 3 × 10^456 (a whole number of 457 digits)"
        );
    }

    #[test]
    fn grouping() {
        assert_eq!(group(369693100), "369,693,100");
        assert_eq!(group(457), "457");
    }
}
