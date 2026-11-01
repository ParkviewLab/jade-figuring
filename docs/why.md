<!--
SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
SPDX-License-Identifier: CC-BY-4.0
-->

# Why jade-figuring is shaped as it is

## The purpose

A small language model answering by voice (Peabody, on google/gemma-4-12b with an 8,192-token context) slips at arithmetic, trigonometry and date sums. jade-figuring gives it tools to which it passes an ordinary expression and from which it receives an exact, speakable result. Providing data is a different job from performing calculations on data types: jade-figuring holds the calculation tools needed now, and is not meant as the home of every mathematics tool.

## Two tools, small definitions

Every tool's name, description and input schema are sent with every request, so in an 8,192-token context their size matters more than their detail. A single tool carrying its own function list and examples measured 1,266 characters as Peabody sends it; the two tools as built measure 762. The descriptions name no notation and no particular kinds of calculation, because the model needs neither (see below), and the definitions are written by hand so that no library adds a `$schema`, titles or descriptions to them.

## The tests that settled the definitions (2026-10-31)

Each test sent twelve spoken-style questions, three runs each, to gemma-4-12b with thinking off, through LM Studio, with the tools offered beside five unrelated ones.

- **No input schema at all.** LM Studio refuses a tool whose `parameters` has no `properties` (HTTP 400). With an empty `properties`, the model called the calculator 23 times out of 36 and passed an empty argument object every time; it never called a tool offered to explain the input.
- **One `expression` string, no description.** The model wrote ordinary calculator notation unprompted: `2^100`, `ln(50)`, `9261^(1/3)`, `0.175 * 2340`, `sin(30 degrees)`, `6 * sin(70 * (pi / 180))`, `52 choose 5`, sometimes LaTeX (`52 \choose 5`, `\frac{52!}{5! \times (52-5)!}`). For dates it had no notation and wrote prose or its own arithmetic into the field; none of the nine date calls could be evaluated.
- **A separate `date_calculator`.** All nine date questions reached it, in short English with ISO dates: `days between 2026-10-31 and 2026-12-25`, `weekday of 2027-07-04`, `July 4, 2027 day of week`, `time between 2026-10-31 09:15 and 2026-11-01 17:40`. A tool was used in 32 runs of 36.
- **"including number bases" in the description.** One of six base-conversion questions reached the tool, against none before, and the overall rate fell to 28 of 36; no measurable effect, so the phrase was left out.

- **The built server, before the first release.** The same twelve questions, three runs each, with the tools answering for real: every one of the 27 calls that reached a tool returned the correct result, which the model then spoke correctly (`9261^(1/3) = 21`, `52! / (5! × (52 - 5)!) = 2598960`, `weekday of 2027-07-04 = Sunday`, `1 day, 8 hours and 25 minutes`). In the other 9 runs the model answered without a tool: correctly for the sine of 30 degrees and the base conversions, but twice wrongly for a height by trigonometry (5.62 against 5.638). Whether the model calls a tool is the model's choice; nothing in the server changes it.

Hence the parsers accept the forms the model actually writes (word forms, `choose`, `52C5`, common LaTeX, plain-English date phrases) rather than asking it to learn one notation, and a named notation was not adopted.

## Results a voice can speak

Engineering calculators display 10 significant digits (the Casio fx-991EX and the TI-36X Pro both do), and so does jade-figuring, which keeps floating-point noise such as 0.30000000000000004 out of a spoken answer. Whole numbers are exact up to 300 digits; beyond that a result would flood the model's context, so it is given to 10 significant digits with its digit count. Each result echoes the expression as it was read, so that the angle unit and the reading of a percentage are visible.

## Percentages

`100 + 3%` is 103: in English a percentage added to a value is a percentage of that value. `3% + 100` has nothing before the percentage to take it of, so the tool answers with the question "3% of what?" rather than guess.

## Dates kept basic

The first version computes days between dates, a date plus or minus days or weeks, the weekday of a date, and the time between two datetimes. Adding months or years needs a rule for month ends (31 January plus a month has no exact answer), and named time zones need a daylight-saving table and its upkeep, so both wait until a need is found.

## No settings

A calculator should answer the same on every call and every deployment. The behaviour is fixed by the requirements and the limits are constants in the build; there is nothing to store, so the container keeps no data.
