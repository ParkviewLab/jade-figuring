<!--
SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
SPDX-License-Identifier: CC-BY-4.0
-->

# Requirements

What jade-figuring must do, as decided on 2026-10-31 before the first version was built. The reasons and the tests behind them are in [`why.md`](why.md).

## The tools

1. The server offers two tools, each with a short description and one string argument, `expression`, declared with no description of its own:
   - `engineering_calculator`: "Evaluates one expression, written as on an engineering calculator."
   - `date_calculator`: "Calculates with dates and times: days or time between two, a date plus or minus a period, the weekday of a date."
2. Neither tool explains its notation, and the descriptions name no particular kinds of calculation: a test adding "including number bases" to `engineering_calculator`'s description made no measurable difference to which questions reached the tool. A test against gemma-4-12b on 2026-10-31 showed that the model writes ordinary calculator notation unprompted (`2^100`, `ln(50)`, `9261^(1/3)`, `6 * sin(70 * (pi / 180))`), and that it needs a declared argument name, without which it sends no expression at all. Each tool therefore accepts the common ways of writing its input, including those the test showed: `sin(30 degrees)` as well as `°` and `deg`, `52 choose 5` and `52 \choose 5` as well as `nCr(52, 5)`, LaTeX such as `\frac{52!}{5! \times (52-5)!}`, `52C5`, and `hex_to_bin(0xFF3A)`; text the model appends after the expression (`= …`, a stray closing brace) is reported, not guessed at. `date_calculator` likewise accepts the plain English the model wrote in the second test on 2026-10-31, with no notation explained: `days between 2026-10-31 and 2026-12-25`, `difference in days between …`, `weekday of 2027-07-04`, `July 4, 2027 day of week`, a bare date (answered with its weekday), and `time between 2026-10-31 09:15 and 2026-11-01 17:40` (with `duration` or `difference` in place of `time`).
3. `engineering_calculator` accepts the common LaTeX forms for everything it computes, since models often write mathematics in LaTeX:
   - **Structure:** `\frac{a}{b}`, `\sqrt{x}`, `\sqrt[n]{x}`, `x^{n}`, `\left(` and `\right)`, `\{ \}` used as grouping braces.
   - **Operators:** `\times`, `\cdot`, `\div`, `\%`, `!`, `\bmod` and `\mod`.
   - **Functions:** `\sin`, `\cos`, `\tan`, `\arcsin`, `\arccos`, `\arctan`, `\sinh`, `\cosh`, `\tanh`, `\ln`, `\log`, `\log_{b}`, `\exp`, `\gcd`, `\min`, `\max`, `\operatorname{name}`.
   - **Grouping functions:** `|x|`, `\lvert x \rvert`, `\lfloor x \rfloor`, `\lceil x \rceil`.
   - **Counting:** `\binom{n}{k}` and `n \choose k`.
   - **Sums, products and calculus:** `\sum_{k=1}^{n}`, `\prod_{k=1}^{n}`, `\int_{a}^{b} f \, dx`, and `\frac{d}{dx}` at a point.
   - **Constants and angles:** `\pi`, `e` and `\mathrm{e}`, and `^\circ` for degrees.
   - **Text:** text in `\text{…}` is not evaluated; an expression that carries it is reported, not guessed at.
4. Together the two definitions, as an MCP client such as Peabody sends them to the model (the chat request's tool list and a line per tool after the system prompt), stay within 800 characters; they measured 762 on 2026-10-31, and a test enforces the ceiling.

## What it computes

5. Arithmetic: addition, subtraction, multiplication and division, accepting both the keyboard and the typographic forms of each (`-` and `−`, `*` and `×`, `/` and `÷`), parentheses, and unary minus.
6. Powers by `^` and by `**`; square roots, cube roots and nth roots.
7. Factorial, written `5!` or as a function; absolute value; rounding to a given number of decimal places, and rounding down and up to whole numbers.
8. Percentages: a percentage added to or subtracted from a value is a percentage of that value (`100 + 3%` is 103, `100 - 3%` is 97); a percentage added to or subtracted from with nothing before it (`3% + 100`) is refused with the question "3% of what?"; anywhere else a percentage is the plain fraction (`8%` is 0.08, `15% * 200` is 30).
9. Logarithms: natural (`ln`), base 10 (`log10`), any base (`log(x, b)`), and `exp`.
10. Trigonometry: `sin`, `cos`, `tan`, their inverses (under both the `asin` and the `arcsin` spellings) and their hyperbolic forms and inverses, with `pi` and `e` as constants.
11. Unmarked angles are in radians; angles marked with `°` or `deg` are read as degrees.

## Engineering calculator functions

The functions an engineering calculator offers on its keys and menus, taken from the TI-36X Pro, beyond those above:

- **Number functions:** integer part, fractional part, greatest integer (floor), ceiling, minimum, maximum, modulo.
- **Number theory:** greatest common divisor, least common multiple, prime factors.
- **Sums and products** over a range (`sum(k^2, k, 1, 10)`).
- **Counting:** combinations (nCr) and permutations (nPr), beside the factorial above.
- **Angles:** degrees, minutes and seconds (`12°31'45"`) and gradians as well as degrees and radians, and conversion of a result to degrees, minutes and seconds.
- **Coordinates:** rectangular to polar and back.
- **Numeric calculus:** the derivative of an expression at a point, and the integral of an expression between two bounds.
- **Physical constants:** the calculator's twenty, by short names (`c`, `g`, `h`, `NA`, `R`, `me`, `mp`, `mn`, `mmu`, `G`, `F`, `a0`, `re`, `k`, `qe`, `u`, `atm`, `eps0`, `mu0`, `ke`; `e` stays Euler's number, so the electron charge is `qe`) (speed of light, gravitational acceleration, Planck's constant, Avogadro's number, the gas constant, the electron, proton, neutron and muon masses, the gravitational constant, Faraday's constant, the Bohr radius, the classical electron radius, Boltzmann's constant, the electron charge, the atomic mass unit, the standard atmosphere, the permittivity and permeability of a vacuum, and Coulomb's constant), at their CODATA values.

## Number bases

- **Conversion** of whole numbers between decimal, hexadecimal, binary and octal, in either direction (`255` to `0xFF`, `0b1010` to `10`).
- **Numbers written in any of the four bases** within an expression, marked by the prefixes `0x`, `0b` and `0o`.
- **Bitwise logic:** and, or, xor, xnor, nand and not, and shifts left and right.
- **Negative numbers** are shown with a minus sign (`-0xFF`); two's complement is given on request for a stated width (8, 16, 32 or 64 bits).

## Results

12. Whole-number results are exact up to 300 digits (`2^100` in full); a larger whole number is given to 10 significant digits with its digit count, so that a result never floods a small model's context.
13. Other results are given to 10 significant digits, as engineering calculators display them, so that a spoken answer never carries floating-point noise: `sin(30°)` gives 0.5 and `0.1 + 0.2` gives 0.3.
14. Each result is accompanied by the expression as it was read (`sin(30°) = 0.5`), so that the model and its user can see what was computed, including the angle unit applied.

## Date arithmetic

Offered by `date_calculator`. In the first version, kept basic until a need for more is found. Dates:

- **Days between** two dates.
- **A date plus or minus** a number of days or weeks.
- **The weekday** of a date.
- **Dates** are written in ISO form (`2026-10-31`), on the Gregorian calendar; no adding of months or years, which would need a rule for month ends.

Datetimes:

- **Time between** two datetimes, given in days, hours, minutes and seconds (`2026-10-31T09:15` to `2026-11-01T17:40` is 1 day, 8 hours, 25 minutes).
- **A datetime plus or minus** a number of days, hours, minutes or seconds.
- **Datetimes** are written in ISO form (`2026-10-31T09:15`, seconds optional), with or without a UTC offset (`Z`, `-07:00`). Two datetimes with offsets are compared exactly; two without are read as clock times in the same place, with no daylight-saving change between them; one of each is refused, since the time between them is unknown. No named time zones (`America/Los_Angeles`).

## Errors

15. An error says what was wrong and where, and suggests the likely correction where one exists ("unknown function 'sine'; use sin", "unbalanced parenthesis at position 12"), so that the model can correct itself on the next round.
16. Mathematically undefined results (division by zero, the logarithm of zero, the square root of a negative number) are reported as such in words, never as Infinity or NaN.

## Safety and limits

17. It never executes code: it accepts only numbers, the listed operators, functions and constants, and nothing else.
18. It refuses runaway work promptly (`9^9^9`, `100000!`, very long expressions) with an error that says why, within a set time and size limit, and stays responsive to other calls.
19. It keeps nothing between calls, and has no settings page: its behaviour is fixed by these requirements and its limits are constants in the build.

## Not in the first version

Statistics and regressions, unit conversions, complex numbers, matrices, vectors, equation solvers, random numbers and exact fractions are left for later versions, to be added when a need is found. Random numbers would also make the same call give different answers.

## Serving and deployment

20. It speaks MCP over streamable HTTP without sessions, at `/mcp`, so that Peabody and LM Studio can reach it.
21. It runs as a container, listening on port 35833 by default, from an image published on GHCR with semver tags and `latest`.
22. It is written entirely in Rust.
23. It is published as ParkviewLab/jade-figuring, a public repository, dual-licensed: AGPL-3.0-or-later or a commercial licence for the code, CC-BY-4.0 for documentation, REUSE-compliant ([`LICENSING.md`](../LICENSING.md)).
