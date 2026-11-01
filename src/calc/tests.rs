// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The requirements' examples and every expression the model wrote in the
//! tests of 2026-10-31, with their expected results.

use super::calculate;

fn result(input: &str) -> String {
    match calculate(input) {
        Ok(s) => s.split_once(" = ").map(|(_, r)| r.to_string()).unwrap(),
        Err(e) => panic!("{input:?} failed: {e}"),
    }
}

fn error(input: &str) -> String {
    match calculate(input) {
        Ok(s) => panic!("{input:?} should fail but gave {s}"),
        Err(e) => e.message,
    }
}

#[track_caller]
fn check(cases: &[(&str, &str)]) {
    for (input, want) in cases {
        assert_eq!(result(input), *want, "for {input:?}");
    }
}

#[test]
fn arithmetic_and_operator_spellings() {
    check(&[
        ("1 + 2 * 3", "7"),
        ("6 × 7", "42"),
        ("8 ÷ 2", "4"),
        ("5 − 3", "2"),
        ("7 / 2", "3.5"),
        ("1 / 3", "0.3333333333"),
        ("-2^2", "-4"),
        ("2^-1", "0.5"),
        ("2 ** 10", "1024"),
        ("2^3^2", "512"),
        ("0.1 + 0.2", "0.3"),
        ("2pi", "6.283185307"),
        ("3(4 + 1)", "15"),
        ("(1 + 2)(3 + 4)", "21"),
        ("2 + 2 =", "4"),
        ("4²", "16"),
    ]);
}

#[test]
fn what_the_model_wrote() {
    check(&[
        ("sin(30 degrees)", "0.5"),
        ("sin(30 deg)", "0.5"),
        ("sin(30°)", "0.5"),
        ("0.175 * 2340", "409.5"),
        ("2340 * 0.175", "409.5"),
        ("2^100", "1267650600228229401496703205376"),
        ("ln(50)", "3.912023005"),
        ("9261^(1/3)", "21"),
        ("52 choose 5", "2598960"),
        ("52! / (5! * 47!)", "2598960"),
        ("\\frac{52!}{5! \\times (52-5)!}", "2598960"),
        ("6 * sin(70 * (pi / 180))", "5.638155725"),
        ("0xFF3A to binary", "0b1111111100111010"),
        ("hex_to_bin(0xFF3A)", "0b1111111100111010"),
        ("hex_to_bin(FF3A)", "0b1111111100111010"),
        ("52C5", "2598960"),
    ]);
}

#[test]
fn appended_text_is_reported() {
    assert!(error("52 \\choose 5}").contains("unmatched closing bracket"));
    assert!(error("52C5 = (52 * 51 * 50 * 49 * 48) / (5 * 4 * 3 * 2 * 1)").contains("after '='"));
    assert!(error("(25-31) + 30 + 31 = ?").contains("after '='"));
    assert!(
        error("52 \\choose 5 \\text{ notation used for combinations}").contains("not evaluated")
    );
}

#[test]
fn powers_roots_and_logarithms() {
    check(&[
        ("sqrt(2)", "1.414213562"),
        ("sqrt(144)", "12"),
        ("cbrt(-8)", "-2"),
        ("root(32, 5)", "2"),
        ("(-8)^(1/3)", "-2"),
        ("ln(e)", "1"),
        ("log10(1000)", "3"),
        ("log(1000)", "3"),
        ("log(8, 2)", "3"),
        ("log_2(8)", "3"),
        ("exp(1)", "2.718281828"),
        ("ln 50", "3.912023005"),
    ]);
}

#[test]
fn factorial_abs_rounding() {
    check(&[
        ("5!", "120"),
        ("factorial(5)", "120"),
        ("0!", "1"),
        ("abs(-4.2)", "4.2"),
        ("|-3|", "3"),
        ("round(2.345, 2)", "2.35"),
        ("round(2.5)", "3"),
        ("floor(-2.5)", "-3"),
        ("ceil(2.1)", "3"),
        ("ipart(-2.7)", "-2"),
        ("fpart(2.75)", "0.75"),
    ]);
}

#[test]
fn percentages() {
    check(&[
        ("100 + 3%", "103"),
        ("100 - 3%", "97"),
        ("8%", "0.08"),
        ("15% * 200", "30"),
        ("17.5% * 2340", "409.5"),
    ]);
    assert!(error("3% + 100").contains("3% of what?"));
}

#[test]
fn trigonometry_and_angles() {
    check(&[
        ("sin(pi/6)", "0.5"),
        ("cos(90°)", "0"),
        ("sin(pi)", "0"),
        ("tan(45°)", "1"),
        ("sin^2(1) + cos^2(1)", "1"),
        ("\\sin^2 1 + \\cos^2 1", "1"),
        ("asin(1)", "1.570796327"),
        ("arcsin(1)", "1.570796327"),
        ("arctan(1)", "0.7853981634"),
        ("tanh(0.5)", "0.4621171573"),
        ("asinh(1)", "0.881373587"),
        ("acosh(2)", "1.316957897"),
        ("sin(100 grad)", "1"),
        ("sin 30°", "0.5"),
        ("pi * 3^2", "28.27433388"),
        ("12°31'45\" + 26°54'38\" to dms", "39°26'23\""),
        ("1.5 to dms", "1°30'0\""),
    ]);
    assert!(error("tan(90°)").contains("undefined"));
}

#[test]
fn engineering_functions() {
    check(&[
        ("gcd(18, 33)", "3"),
        ("lcm(6, 9)", "18"),
        ("factors(253)", "11 × 23"),
        ("factors(360)", "2^3 × 3^2 × 5"),
        ("nCr(52, 5)", "2598960"),
        ("nPr(8, 3)", "336"),
        ("mod(17, 12)", "5"),
        ("17 mod 12", "5"),
        ("mod(-1, 3)", "2"),
        ("min(4, -5)", "-5"),
        ("max(0.6, 0.7)", "0.7"),
        ("sum(k^2, k, 1, 10)", "385"),
        ("prod(k, k, 1, 5)", "120"),
        ("deriv(x^2, x, 3)", "6"),
        ("integral(x^2, x, 0, 3)", "9"),
        ("integral(sin(x), x, 0, pi)", "2"),
        ("polar(3, 4)", "r = 5, θ = 0.927295218 (53.13010235°)"),
        ("rect(5, 0.927295218)", "x = 3, y = 4"),
        ("c", "299792458"),
        ("qe", "1.602176634 × 10^-19"),
        ("h * c", "1.986445857 × 10^-25"),
    ]);
}

#[test]
fn number_bases() {
    check(&[
        ("255 to hex", "0xFF"),
        ("255 in hex", "0xFF"),
        ("hex(255)", "0xFF"),
        ("0xFF to decimal", "255"),
        ("0xFF", "255"),
        ("0b1010 xor 0b0110", "12"),
        ("0b1010 xor 0b0110 to binary", "0b1100"),
        ("0b1111 and 0b1010", "10"),
        ("192 nand 48", "-1"),
        ("1 << 10", "1024"),
        ("-255 to hex", "-0xFF"),
        ("twos(-1, 8)", "0b11111111 (0xFF, 8 bits)"),
        ("dec_to_hex(255)", "0xFF"),
        ("bin_to_dec(1010)", "10"),
        ("0o17 to dec", "15"),
    ]);
}

#[test]
fn latex() {
    check(&[
        ("\\sqrt{16}", "4"),
        ("\\sqrt[3]{27}", "3"),
        ("2^{10}", "1024"),
        ("\\left(1 + 2\\right) \\cdot 3", "9"),
        ("\\frac{1}{2}", "0.5"),
        ("\\frac12", "0.5"),
        ("\\sin(30^\\circ)", "0.5"),
        ("\\sin 30^{\\circ}", "0.5"),
        ("\\cos(\\pi)", "-1"),
        ("\\ln(\\mathrm{e})", "1"),
        ("\\log_{2}{8}", "3"),
        ("\\binom{52}{5}", "2598960"),
        ("{52 \\choose 5}", "2598960"),
        ("\\lvert -3 \\rvert", "3"),
        ("\\lfloor 2.7 \\rfloor", "2"),
        ("\\lceil 2.1 \\rceil", "3"),
        ("17 \\bmod 5", "2"),
        ("\\sum_{k=1}^{10} k^2", "385"),
        ("\\prod_{k=1}^{5} k", "120"),
        ("\\int_{0}^{3} x^2 \\, dx", "9"),
        ("\\frac{d}{dx} x^3 \\Big|_{x=2}", "12"),
        ("\\operatorname{gcd}(18, 33)", "3"),
        ("$2 \\times 3$", "6"),
        ("50\\%", "0.5"),
        ("10 \\div 4", "2.5"),
    ]);
}

#[test]
fn results_and_large_numbers() {
    check(&[
        (
            "2^1000",
            "about 1.071508607 × 10^301 (a whole number of 302 digits)",
        ),
        ("1e20", "1 × 10^20"),
        ("10^15 + 0.5", "1 × 10^15"),
    ]);
    assert_eq!(calculate("sin(30)").unwrap(), "sin(30 rad) = -0.9880316241");
    assert_eq!(calculate("sin(pi/6)").unwrap(), "sin((pi / 6) rad) = 0.5");
    let read = calculate("sin(30°)").unwrap();
    assert_eq!(read, "sin(30°) = 0.5");
    assert_eq!(calculate("log(50)").unwrap(), "log10(50) = 1.698970004");
    assert_eq!(
        calculate("\\frac{52!}{5! \\times (52-5)!}").unwrap(),
        "52! / (5! × (52 - 5)!) = 2598960"
    );
}

#[test]
fn errors_say_what_and_where() {
    assert!(error("sinn(1)").contains("use sin"));
    assert!(error("squirt(2)").contains("use sqrt"));
    assert!(error("(1 + 2").contains("not closed"));
    assert!(error("1 / 0").contains("division by zero"));
    assert!(error("ln(0)").contains("logarithm of zero"));
    assert!(error("sqrt(-1)").contains("not a real number"));
    assert!(error("asin(2)").contains("between -1 and 1"));
    assert!(error("2.5!").contains("whole number"));
    assert!(error("").contains("empty"));
    assert!(error("2 +").contains("ends"));
    assert!(error("import os").contains("unknown"));
    assert!(error("x + 1").contains("only inside"));
    assert!(error("e^i").contains("complex"));
    assert!(error("1 000").contains("two numbers"));
    assert!(error("1,000 + 1").contains("thousands"));
    assert!(error("0b102").contains("base 2"));
    assert!(error("52 C 5").contains("unknown name 'C'"));
}

#[test]
fn found_in_review() {
    for e in [
        "1e999",
        "-1e999",
        "sin(1e999)",
        "sin(10^400)",
        "(10^400)°",
        "(10^400)%",
        "floor(1e999)",
    ] {
        assert!(
            error(e).contains("too large") || error(e).contains("undefined"),
            "{e}: {}",
            error(e)
        );
    }
    check(&[
        ("round(1.5, 400)", "1.5"),
        ("round(1.5, -400)", "0"),
        ("sum(k, k, 10^15, 10^15)", "1000000000000000"),
        ("(-1)^(10^20)", "1"),
        ("(-1)^(10^20 + 1)", "-1"),
        ("1^(10^30)", "1"),
        ("0^0", "1"),
        ("sin(1e-20)", "1 × 10^-20"),
        ("tan(1e-20)", "1 × 10^-20"),
        ("(-8)^(2/3)", "4"),
        ("(-8)^(1/3)", "-2"),
        ("(-32)^(3/5)", "-8"),
    ]);
    assert!(result("2^17000").contains("5,118 digits"));
    assert!(result("2^20000").contains("6,021 digits"));
    assert!(error("sum(k, k, 10^20, 10^20)").contains("bounds"));
    assert!(error("sum(k, k, -2^63, 0)").contains("terms"));
    assert!(error("prod(k, k, 1, 100000)").contains("digits"));
    assert!(error("-5% + 100").contains("of what"));
    assert!(error(&format!("{}1{}", "(".repeat(300), ")".repeat(300))).contains("nests"));
}

#[test]
fn runaway_work_is_refused_promptly() {
    let started = std::time::Instant::now();
    assert!(error("9^9^9").contains("digits"));
    assert!(error("100000!").contains("digits"));
    assert!(error("sum(k, k, 1, 1000000)").contains("terms"));
    assert!(error(&"1+".repeat(600)).contains("limit"));
    assert!(started.elapsed() < std::time::Duration::from_secs(2));
}
