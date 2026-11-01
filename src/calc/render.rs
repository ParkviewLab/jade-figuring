// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The expression as it was read, in plain calculator notation, so that the
//! reader sees what was computed: `\frac{52!}{5! \times 47!}` reads back as
//! `52! / (5! × 47!)`, and `log(50)` as `log10(50)`.

use super::functions::canonical;
use super::parse::{BinOp, Expr, Target, Unit};

fn bp(e: &Expr) -> u8 {
    match e {
        Expr::Convert(..) => 1,
        Expr::Bin(op, ..) => match op {
            BinOp::Choose => 2,
            BinOp::Or | BinOp::Xor | BinOp::Xnor => 3,
            BinOp::And | BinOp::Nand => 4,
            BinOp::Shl | BinOp::Shr => 5,
            BinOp::Add | BinOp::Sub => 6,
            BinOp::Mul | BinOp::Div | BinOp::Mod => 7,
            BinOp::Pow => 9,
        },
        Expr::Neg(_) | Expr::Not(_) => 8,
        _ => 10,
    }
}

fn wrap(e: &Expr, min: u8) -> String {
    let s = render(e);
    if bp(e) < min { format!("({s})") } else { s }
}

/// Whether an angle unit appears anywhere in `e`.
pub fn has_angle(e: &Expr) -> bool {
    match e {
        Expr::Angle(..) | Expr::Dms(..) => true,
        Expr::Neg(x) | Expr::Not(x) | Expr::Factorial(x) | Expr::Percent(x) => has_angle(x),
        Expr::Bin(_, a, b) => has_angle(a) || has_angle(b),
        Expr::Call(_, args) => args.iter().any(has_angle),
        Expr::Convert(x, _) => has_angle(x),
        Expr::Num(_) | Expr::Ident(_) => false,
    }
}

pub fn render(e: &Expr) -> String {
    match e {
        Expr::Num(n) => match n.base {
            16 => format!("0x{}", n.text.to_uppercase()),
            2 => format!("0b{}", n.text),
            8 => format!("0o{}", n.text),
            _ => n.text.clone(),
        },
        Expr::Ident(s) => s.clone(),
        Expr::Neg(x) => format!("-{}", wrap(x, 9)),
        Expr::Not(x) => format!("not {}", wrap(x, 9)),
        Expr::Factorial(x) => format!("{}!", wrap(x, 10)),
        Expr::Percent(x) => format!("{}%", wrap(x, 10)),
        Expr::Angle(x, u) => match u {
            Unit::Degrees => format!("{}°", wrap(x, 10)),
            Unit::Radians => format!("{} rad", wrap(x, 10)),
            Unit::Gradians => format!("{} grad", wrap(x, 10)),
        },
        Expr::Dms(d, m, s) => format!("{}°{}'{}\"", render(d), render(m), render(s)),
        Expr::Bin(op, a, b) => {
            let me = bp(e);
            let sym = match op {
                BinOp::Add => " + ",
                BinOp::Sub => " - ",
                BinOp::Mul => " × ",
                BinOp::Div => " / ",
                BinOp::Pow => "^",
                BinOp::Mod => " mod ",
                BinOp::Choose => " choose ",
                BinOp::And => " and ",
                BinOp::Or => " or ",
                BinOp::Xor => " xor ",
                BinOp::Xnor => " xnor ",
                BinOp::Nand => " nand ",
                BinOp::Shl => " << ",
                BinOp::Shr => " >> ",
            };
            // Power is right-associative; the rest are left-associative.
            let (lmin, rmin) = if *op == BinOp::Pow {
                (me + 1, me)
            } else {
                (me, me + 1)
            };
            format!("{}{sym}{}", wrap(a, lmin), wrap(b, rmin))
        }
        Expr::Call(name, args) => {
            let shown = match (canonical(name), args.len()) {
                (Some("log"), 1) => "log10".to_string(),
                (Some("ncr"), _) => "nCr".to_string(),
                (Some("npr"), _) => "nPr".to_string(),
                (Some(c), _) => c.to_string(),
                (None, _) => name.clone(),
            };
            let trig = matches!(
                shown.as_str(),
                "sin" | "cos" | "tan" | "sec" | "csc" | "cot" | "rect"
            );
            let args: Vec<String> = args
                .iter()
                .enumerate()
                .map(|(k, a)| {
                    let angle = (shown == "rect") == (k == 1);
                    if trig && angle && !has_angle(a) {
                        format!("{} rad", wrap(a, 10))
                    } else {
                        render(a)
                    }
                })
                .collect();
            format!("{shown}({})", args.join(", "))
        }
        Expr::Convert(x, t) => {
            let t = match t {
                Target::Hex => "hex",
                Target::Bin => "binary",
                Target::Oct => "octal",
                Target::Dec => "decimal",
                Target::Dms => "dms",
            };
            format!("{} to {t}", wrap(x, 2))
        }
    }
}
