// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The date calculator: days or time between two dates or datetimes, a date
//! plus or minus a period, and the weekday of a date, from the short English
//! a model writes ("days between 2026-10-31 and 2026-12-25").

use jiff::civil::{Date, DateTime, Time};

/// Requests longer than this are refused.
pub const MAX_REQUEST_CHARS: usize = 500;

#[derive(Debug, Clone, PartialEq)]
pub struct DateError {
    pub message: String,
}

impl DateError {
    fn new(message: impl Into<String>) -> Self {
        DateError {
            message: message.into(),
        }
    }
}

impl std::fmt::Display for DateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

/// A date or datetime as written, with the UTC offset if one was given.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Moment {
    dt: DateTime,
    has_time: bool,
    /// Seconds east of UTC.
    offset: Option<i32>,
}

impl Moment {
    /// Seconds since the epoch: in UTC where an offset was given, else on the
    /// clock as written.
    fn seconds(&self) -> i64 {
        let d = self.dt.date();
        let days = days_from_civil(d.year() as i64, d.month() as i64, d.day() as i64);
        let t = self.dt.time();
        let secs =
            days * 86_400 + t.hour() as i64 * 3600 + t.minute() as i64 * 60 + t.second() as i64;
        secs - self.offset.unwrap_or(0) as i64
    }

    fn show(&self) -> String {
        let d = self.dt.date();
        let mut s = format!("{:04}-{:02}-{:02}", d.year(), d.month(), d.day());
        if self.has_time {
            let t = self.dt.time();
            s.push_str(&format!(" {:02}:{:02}", t.hour(), t.minute()));
            if t.second() != 0 {
                s.push_str(&format!(":{:02}", t.second()));
            }
        }
        if let Some(o) = self.offset {
            if o == 0 {
                s.push('Z');
            } else {
                let sign = if o < 0 { '-' } else { '+' };
                s.push_str(&format!(
                    "{sign}{:02}:{:02}",
                    o.abs() / 3600,
                    o.abs() % 3600 / 60
                ));
            }
        }
        s
    }
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Howard Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Unit {
    Seconds,
    Minutes,
    Hours,
    Days,
    Weeks,
}

impl Unit {
    fn seconds(self) -> i64 {
        match self {
            Unit::Seconds => 1,
            Unit::Minutes => 60,
            Unit::Hours => 3600,
            Unit::Days => 86_400,
            Unit::Weeks => 604_800,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Piece {
    Moment(Moment),
    /// A period of `n` units, signed by the word before it (`plus`, `minus`).
    Period(i64, Unit),
    Word(String),
    Plus,
    Minus,
}

const MONTHS: [&str; 12] = [
    "january",
    "february",
    "march",
    "april",
    "may",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

fn month_of(word: &str) -> Option<i8> {
    let w = word.trim_end_matches('.');
    if w.len() < 3 {
        return None;
    }
    MONTHS
        .iter()
        .position(|m| m.starts_with(w) && (w.len() >= 3))
        .map(|i| i as i8 + 1)
}

fn unit_of(word: &str) -> Option<Result<Unit, DateError>> {
    Some(Ok(match word {
        "s" | "sec" | "secs" | "second" | "seconds" => Unit::Seconds,
        "min" | "mins" | "minute" | "minutes" => Unit::Minutes,
        "h" | "hr" | "hrs" | "hour" | "hours" => Unit::Hours,
        "d" | "day" | "days" => Unit::Days,
        "w" | "wk" | "wks" | "week" | "weeks" => Unit::Weeks,
        "month" | "months" | "year" | "years" | "yr" | "yrs" => {
            return Some(Err(DateError::new(
                "adding months or years is not supported, since a month's length varies (31 January plus a month has no exact answer); add days or weeks instead",
            )));
        }
        _ => return None,
    }))
}

/// Splits the request into words, keeping the characters dates and times use.
fn words(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    for c in input.chars() {
        let keep = c.is_alphanumeric() || matches!(c, '-' | ':' | '/' | '.' | '+' | '−');
        if keep {
            cur.push(c);
        } else {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
            if c == ',' {
                out.push(",".into());
            }
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

fn bad_date(text: &str) -> DateError {
    DateError::new(format!("'{text}' is not a valid date"))
}

fn make_date(y: i64, m: i64, d: i64, text: &str) -> Result<Date, DateError> {
    let (Ok(y), Ok(m), Ok(d)) = (i16::try_from(y), i8::try_from(m), i8::try_from(d)) else {
        return Err(bad_date(text));
    };
    Date::new(y, m, d).map_err(|_| bad_date(text))
}

/// `2026-10-31`, optionally with `T09:15[:30]` and an offset `Z` or `-07:00`.
fn iso(word: &str) -> Result<Option<Moment>, DateError> {
    let bytes = word.as_bytes();
    if word.len() < 10
        || !word.is_char_boundary(10)
        || !word[..10].is_ascii()
        || !(bytes[4] == b'-' && bytes[7] == b'-')
        || !word[..4].chars().all(|c| c.is_ascii_digit())
    {
        return Ok(None);
    }
    let (y, m, d) = (&word[..4], &word[5..7], &word[8..10]);
    if ![m, d].iter().all(|p| p.chars().all(|c| c.is_ascii_digit())) {
        return Ok(None);
    }
    let date = make_date(
        y.parse().unwrap(),
        m.parse().unwrap(),
        d.parse().unwrap(),
        word,
    )?;
    let rest = &word[10..];
    if rest.is_empty() {
        return Ok(Some(Moment {
            dt: date.to_datetime(Time::midnight()),
            has_time: false,
            offset: None,
        }));
    }
    let Some(rest) = rest.strip_prefix('T').or_else(|| rest.strip_prefix('t')) else {
        return Err(bad_date(word));
    };
    let (time, offset) = split_offset(rest, word)?;
    let t = clock(time, word)?;
    Ok(Some(Moment {
        dt: date.to_datetime(t),
        has_time: true,
        offset,
    }))
}

fn split_offset<'a>(s: &'a str, whole: &str) -> Result<(&'a str, Option<i32>), DateError> {
    if let Some(t) = s.strip_suffix('Z').or_else(|| s.strip_suffix('z')) {
        return Ok((t, Some(0)));
    }
    if let Some(i) = s.rfind(['+', '-']) {
        let (t, o) = s.split_at(i);
        return Ok((t, Some(offset(o).ok_or_else(|| bad_date(whole))?)));
    }
    Ok((s, None))
}

/// `-07:00`, `+0530`, `+05`.
fn offset(o: &str) -> Option<i32> {
    let sign = if o.starts_with('-') { -1 } else { 1 };
    let digits: String = o[1..].chars().filter(|c| c.is_ascii_digit()).collect();
    let (h, m) = match digits.len() {
        2 => (digits.parse::<i32>().ok()?, 0),
        4 => (
            digits[..2].parse::<i32>().ok()?,
            digits[2..].parse::<i32>().ok()?,
        ),
        _ => return None,
    };
    if h > 18 || m > 59 {
        return None;
    }
    Some(sign * (h * 3600 + m * 60))
}

/// `09:15`, `9:15:30`.
fn clock(s: &str, whole: &str) -> Result<Time, DateError> {
    let parts: Vec<&str> = s.split(':').collect();
    if !(2..=3).contains(&parts.len())
        || parts
            .iter()
            .any(|p| p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()))
    {
        return Err(DateError::new(format!("'{whole}' is not a valid time")));
    }
    let h: i8 = parts[0].parse().unwrap_or(99);
    let m: i8 = parts[1].parse().unwrap_or(99);
    let sec: i8 = parts.get(2).map(|p| p.parse().unwrap_or(99)).unwrap_or(0);
    Time::new(h, m, sec, 0).map_err(|_| DateError::new(format!("'{whole}' is not a valid time")))
}

/// A time at word `k`: `09:15`, `9:15 pm`, `09:15-07:00`, `09:15Z`, `17:40 UTC`.
/// Returns the time, its offset, and the number of words it took.
fn time_at(ws: &[String], k: usize) -> Result<Option<(Time, Option<i32>, usize)>, DateError> {
    let at = |j: usize| ws.get(j).map(String::as_str).unwrap_or("");
    let word = at(k);
    if !word.contains(':') || !word.starts_with(|c: char| c.is_ascii_digit()) {
        return Ok(None);
    }
    let (text, mut off) = split_offset(word, word)?;
    if !is_clock(text) {
        return Ok(None);
    }
    let mut t = clock(text, word)?;
    let mut used = 1;
    match at(k + 1) {
        "am" | "a.m." => {
            if t.hour() > 12 {
                return Err(DateError::new(format!("'{word} am' is not a valid time")));
            }
            if t.hour() == 12 {
                t = Time::new(0, t.minute(), t.second(), 0).unwrap();
            }
            used += 1;
        }
        "pm" | "p.m." => {
            if t.hour() > 12 {
                return Err(DateError::new(format!("'{word} pm' is not a valid time")));
            }
            if t.hour() < 12 {
                t = Time::new(t.hour() + 12, t.minute(), t.second(), 0).unwrap();
            }
            used += 1;
        }
        _ => {}
    }
    if off.is_none() {
        let next = at(k + used);
        if matches!(next, "z" | "utc" | "gmt") {
            off = Some(0);
            used += 1;
        } else if next.starts_with(['+', '-']) && offset(next).is_some() {
            off = offset(next);
            used += 1;
        }
    }
    Ok(Some((t, off, used)))
}

fn is_clock(w: &str) -> bool {
    w.contains(':')
        && w.split(':')
            .all(|p| !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()))
}

fn ordinal(w: &str) -> Option<i64> {
    let digits = w
        .trim_end_matches("st")
        .trim_end_matches("nd")
        .trim_end_matches("rd")
        .trim_end_matches("th");
    if digits.is_empty() || digits.len() > 2 || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

fn year(w: &str) -> Option<i64> {
    (w.len() == 4 && w.chars().all(|c| c.is_ascii_digit())).then(|| w.parse().unwrap())
}

/// Reads the request into dates, periods and words.
fn pieces(input: &str) -> Result<Vec<Piece>, DateError> {
    let ws: Vec<String> = words(&input.to_lowercase());
    let mut out = Vec::new();
    let mut i = 0;
    let at = |k: usize| ws.get(k).map(String::as_str).unwrap_or("");
    while i < ws.len() {
        let w = ws[i].as_str();
        if w == "," {
            i += 1;
            continue;
        }
        if w.contains('/') && w.chars().filter(|c| *c == '/').count() == 2 {
            return Err(DateError::new(format!(
                "'{w}' could be read more than one way; write dates as 2026-10-31"
            )));
        }
        if let Some(m) = iso(w)? {
            // `2026-10-31 09:15`, with an optional offset or am/pm.
            let mut m = m;
            i += 1;
            if !m.has_time {
                let k = if at(i) == "at" { i + 1 } else { i };
                if let Some((t, off, used)) = time_at(&ws, k)? {
                    m = Moment {
                        dt: m.dt.date().to_datetime(t),
                        has_time: true,
                        offset: off,
                    };
                    i = k + used;
                }
            } else if m.offset.is_none() {
                if matches!(at(i), "z" | "utc" | "gmt") {
                    m.offset = Some(0);
                    i += 1;
                } else if at(i).starts_with(['+', '-']) && offset(at(i)).is_some() {
                    m.offset = offset(at(i));
                    i += 1;
                }
            }
            out.push(Piece::Moment(m));
            continue;
        }
        // `July 4, 2027`, `4 July 2027`, `the 4th of July 2027`.
        let written = if let (Some(mo), Some(d), Some(y)) = (
            month_of(w),
            ordinal(at(i + 1)),
            year(at(i + 2)).or(year(at(i + 3))),
        ) {
            let step = if year(at(i + 2)).is_some() { 3 } else { 4 };
            Some((mo, d, y, step))
        } else if let (Some(d), Some(mo)) = (ordinal(w), month_of(at(i + 1))) {
            year(at(i + 2)).map(|y| (mo, d, y, 3))
        } else if let (Some(d), "of", Some(mo)) = (ordinal(w), at(i + 1), month_of(at(i + 2))) {
            year(at(i + 3)).map(|y| (mo, d, y, 4))
        } else {
            None
        };
        if let Some((mo, d, y, step)) = written {
            let text = ws[i..i + step].join(" ");
            let date = make_date(y, mo as i64, d, &text)?;
            let mut m = Moment {
                dt: date.to_datetime(Time::midnight()),
                has_time: false,
                offset: None,
            };
            i += step;
            // An optional time: `at 9:15 am`, `17:40`.
            let k = if at(i) == "at" { i + 1 } else { i };
            if let Some((t, off, used)) = time_at(&ws, k)? {
                m.dt = date.to_datetime(t);
                m.has_time = true;
                m.offset = off;
                i = k + used;
            }
            out.push(Piece::Moment(m));
            continue;
        }
        if matches!(w, "today" | "now" | "tomorrow" | "yesterday" | "tonight") {
            return Err(DateError::new(format!(
                "'{w}' is not known here; find the current date first, then pass it as 2026-10-31"
            )));
        }
        // `3 weeks`, `+3 days`, `-2h`.
        let (num_word, unit_word, step) = match w.find(|c: char| c.is_alphabetic()) {
            Some(p)
                if p > 0
                    && w[..p]
                        .trim_start_matches(['+', '-', '−'])
                        .chars()
                        .all(|c| c.is_ascii_digit()) =>
            {
                (&w[..p], &w[p..], 1)
            }
            _ => (w, at(i + 1), 2),
        };
        let signless = num_word.trim_start_matches(['+', '-', '−']);
        if !signless.is_empty()
            && signless.chars().all(|c| c.is_ascii_digit())
            && let Some(unit) = unit_of(unit_word)
        {
            let unit = unit?;
            if num_word.starts_with(['+']) {
                out.push(Piece::Plus);
            } else if num_word.starts_with(['-', '−']) {
                out.push(Piece::Minus);
            }
            let n: i64 = signless
                .parse()
                .map_err(|_| DateError::new(format!("'{w}' is too large")))?;
            if n > 10_000_000 {
                return Err(DateError::new(format!(
                    "{n} {unit_word} is too long a period"
                )));
            }
            out.push(Piece::Period(n, unit));
            i += step;
            continue;
        }
        match w {
            "+" | "plus" | "add" | "after" | "later" => out.push(Piece::Plus),
            "-" | "−" | "minus" | "subtract" | "less" | "before" | "earlier" | "ago" => {
                out.push(Piece::Minus)
            }
            _ => out.push(Piece::Word(w.to_string())),
        }
        i += 1;
    }
    Ok(out)
}

fn weekday(d: Date) -> &'static str {
    use jiff::civil::Weekday::*;
    match d.weekday() {
        Monday => "Monday",
        Tuesday => "Tuesday",
        Wednesday => "Wednesday",
        Thursday => "Thursday",
        Friday => "Friday",
        Saturday => "Saturday",
        Sunday => "Sunday",
    }
}

fn plural(n: i64, what: &str) -> String {
    let mut s = format!("{} {what}", group(n));
    if n != 1 {
        s.push('s');
    }
    s
}

fn group(n: i64) -> String {
    let s = n.abs().to_string();
    let mut out = String::new();
    for (k, c) in s.chars().enumerate() {
        if k > 0 && (s.len() - k).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

fn join_parts(parts: &[String]) -> String {
    match parts.len() {
        0 => "0 seconds".into(),
        1 => parts[0].clone(),
        _ => format!(
            "{} and {}",
            parts[..parts.len() - 1].join(", "),
            parts[parts.len() - 1]
        ),
    }
}

fn duration(secs: i64, with_time: bool) -> String {
    let s = secs.abs();
    if !with_time {
        let days = s / 86_400;
        let mut out = plural(days, "day");
        if days >= 7 {
            let (w, d) = (days / 7, days % 7);
            let mut p = vec![plural(w, "week")];
            if d > 0 {
                p.push(plural(d, "day"));
            }
            out.push_str(&format!(" ({})", join_parts(&p)));
        }
        return out;
    }
    let (d, h, m, sec) = (s / 86_400, s % 86_400 / 3600, s % 3600 / 60, s % 60);
    let mut p = Vec::new();
    if d > 0 {
        p.push(plural(d, "day"));
    }
    if h > 0 {
        p.push(plural(h, "hour"));
    }
    if m > 0 {
        p.push(plural(m, "minute"));
    }
    if sec > 0 {
        p.push(plural(sec, "second"));
    }
    let mut out = join_parts(&p);
    if d > 0 || h > 0 {
        let total_min = s / 60;
        out.push_str(&format!(" ({} in all)", plural(total_min, "minute")));
    }
    out
}

/// Answers a date request with `<request as read> = <result>`.
pub fn calculate(input: &str) -> Result<String, DateError> {
    let input = input.trim();
    if input.chars().count() > MAX_REQUEST_CHARS {
        return Err(DateError::new(format!(
            "the request is longer than {MAX_REQUEST_CHARS} characters"
        )));
    }
    let ps = pieces(input)?;
    let moments: Vec<Moment> = ps
        .iter()
        .filter_map(|p| {
            if let Piece::Moment(m) = p {
                Some(*m)
            } else {
                None
            }
        })
        .collect();
    let has_period = ps.iter().any(|p| matches!(p, Piece::Period(..)));
    match (moments.len(), has_period) {
        (0, _) => Err(DateError::new(
            "no date found; write dates as 2026-10-31 (with a time as 2026-10-31 09:15), as in: days between 2026-10-31 and 2026-12-25",
        )),
        (2, false) => {
            // `B - A` is the time from A to B.
            let first = ps
                .iter()
                .position(|p| matches!(p, Piece::Moment(_)))
                .unwrap();
            let second = ps
                .iter()
                .rposition(|p| matches!(p, Piece::Moment(_)))
                .unwrap();
            if ps[first..second].iter().any(|p| matches!(p, Piece::Minus)) {
                between(moments[1], moments[0])
            } else {
                between(moments[0], moments[1])
            }
        }
        (1, false) => {
            let m = moments[0];
            Ok(format!(
                "weekday of {} = {}",
                m.show(),
                weekday(m.dt.date())
            ))
        }
        (1, true) => shifted(&ps, moments[0]),
        _ => Err(DateError::new(
            "send one calculation at a time: two dates for the time between them, or one date with a period to add or subtract",
        )),
    }
}

fn between(a: Moment, b: Moment) -> Result<String, DateError> {
    if a.offset.is_some() != b.offset.is_some() && (a.has_time || b.has_time) {
        return Err(DateError::new(
            "one datetime has a UTC offset and the other has none, so the time between them is unknown; give both an offset or neither",
        ));
    }
    let diff = b.seconds() - a.seconds();
    let with_time = a.has_time || b.has_time;
    let mut text = duration(diff, with_time);
    if diff < 0 {
        text.push_str(", the second being the earlier");
    }
    Ok(format!("from {} to {} = {text}", a.show(), b.show()))
}

fn shifted(ps: &[Piece], m: Moment) -> Result<String, DateError> {
    // The sign of each period is the nearest `plus` or `minus` word before it,
    // or after it for `3 days before`, `2 weeks ago`.
    let mut total: i64 = 0;
    let mut read = Vec::new();
    let mut fine = false;
    let mut previous: Option<i64> = None;
    for (k, p) in ps.iter().enumerate() {
        let Piece::Period(n, unit) = p else { continue };
        if *unit != Unit::Days && *unit != Unit::Weeks {
            fine = true;
        }
        // The sign written before this period, back to the previous period or the date.
        let before = ps[..k].iter().rev().find_map(|q| match q {
            Piece::Plus => Some(Some(1)),
            Piece::Minus => Some(Some(-1)),
            Piece::Period(..) | Piece::Moment(_) => Some(None),
            Piece::Word(_) => None,
        });
        // A sign written after it (`3 days before`, `2 weeks and 3 days ago`), up to the date.
        let after = ps[k + 1..].iter().find_map(|q| match q {
            Piece::Plus => Some(Some(1)),
            Piece::Minus => Some(Some(-1)),
            Piece::Moment(_) => Some(None),
            Piece::Period(..) | Piece::Word(_) => None,
        });
        let sign = match (before.flatten(), previous, after.flatten()) {
            (Some(s), _, _) => s,
            (None, Some(s), _) => s,
            (None, None, Some(s)) => s,
            _ => 1,
        };
        previous = Some(sign);
        total += sign * n * unit.seconds();
        let unit_name = match unit {
            Unit::Seconds => "second",
            Unit::Minutes => "minute",
            Unit::Hours => "hour",
            Unit::Days => "day",
            Unit::Weeks => "week",
        };
        read.push(format!(
            "{} {}",
            if sign < 0 { "-" } else { "+" },
            plural(*n, unit_name)
        ));
    }
    if fine && !m.has_time {
        return Err(DateError::new(
            "hours, minutes and seconds are added to a datetime, as in 2026-10-31 09:15 plus 8 hours",
        ));
    }
    let secs = m.seconds() + m.offset.unwrap_or(0) as i64 + total;
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let date = civil_from_days(days)
        .ok_or_else(|| DateError::new("the result is outside the years this calculator handles"))?;
    let time = Time::new(
        (rem / 3600) as i8,
        (rem % 3600 / 60) as i8,
        (rem % 60) as i8,
        0,
    )
    .unwrap();
    let result = Moment {
        dt: date.to_datetime(time),
        has_time: m.has_time,
        offset: m.offset,
    };
    Ok(format!(
        "{} {} = {} ({})",
        m.show(),
        read.join(" "),
        result.show(),
        weekday(date)
    ))
}

fn civil_from_days(z: i64) -> Option<Date> {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    Date::new(i16::try_from(y).ok()?, m as i8, d as i8).ok()
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn what_the_model_wrote() {
        let cases = [
            (
                "days between 2026-10-31 and 2026-12-25",
                "55 days (7 weeks and 6 days)",
            ),
            (
                "difference in days between 2026-10-31 and 2026-12-25",
                "55 days (7 weeks and 6 days)",
            ),
            ("July 4, 2027", "Sunday"),
            ("July 4, 2027 day of week", "Sunday"),
            ("weekday of July 4, 2027", "Sunday"),
            ("weekday of 2027-07-04", "Sunday"),
            (
                "time between 2026-10-31 09:15 and 2026-11-01 17:40",
                "1 day, 8 hours and 25 minutes (1,945 minutes in all)",
            ),
            (
                "duration between 2026-10-31 09:15 and 2026-11-01 17:40",
                "1 day, 8 hours and 25 minutes (1,945 minutes in all)",
            ),
            (
                "difference between 2026-10-31 09:15 and 2026-11-01 17:40",
                "1 day, 8 hours and 25 minutes (1,945 minutes in all)",
            ),
        ];
        for (input, want) in cases {
            assert_eq!(result(input), want, "for {input:?}");
        }
    }

    #[test]
    fn written_forms() {
        assert_eq!(result("4 July 2027"), "Sunday");
        assert_eq!(result("the 4th of July 2027"), "Sunday");
        assert_eq!(result("Dec 25, 2026"), "Friday");
        assert_eq!(
            result("2026-10-31T09:15 to 2026-10-31T17:40"),
            "8 hours and 25 minutes (505 minutes in all)"
        );
        assert_eq!(
            result("from October 31, 2026 at 9:15 am to November 1, 2026 at 5:40 pm"),
            "1 day, 8 hours and 25 minutes (1,945 minutes in all)"
        );
        assert_eq!(
            result("date(\"2026-12-25\") - date(\"2026-10-31\")"),
            "55 days (7 weeks and 6 days)"
        );
        assert_eq!(
            result("2026-10-31 to 2026-10-01"),
            "30 days (4 weeks and 2 days), the second being the earlier"
        );
    }

    #[test]
    fn adding_and_subtracting() {
        assert_eq!(result("2026-10-31 plus 3 weeks"), "2026-11-21 (Saturday)");
        assert_eq!(result("2026-10-31 + 3 weeks"), "2026-11-21 (Saturday)");
        assert_eq!(result("3 weeks after 2026-10-31"), "2026-11-21 (Saturday)");
        assert_eq!(result("10 days before 2026-12-25"), "2026-12-15 (Tuesday)");
        assert_eq!(result("2026-12-25 minus 10 days"), "2026-12-15 (Tuesday)");
        assert_eq!(
            result("2026-10-31 + 2 weeks and 3 days"),
            "2026-11-17 (Tuesday)"
        );
        assert_eq!(
            result("2026-10-31 09:15 plus 8 hours"),
            "2026-10-31 17:15 (Saturday)"
        );
        assert_eq!(
            result("2026-10-31T23:30-07:00 + 45 minutes"),
            "2026-11-01 00:15-07:00 (Sunday)"
        );
        assert_eq!(result("2024-02-28 + 1 day"), "2024-02-29 (Thursday)");
    }

    #[test]
    fn found_in_review() {
        assert!(error("2026-10-3é").contains("no date") || error("2026-10-3é").contains("valid"));
        assert_eq!(
            result("time between 2026-10-31 09:15-07:00 and 2026-11-01 17:40-07:00"),
            "1 day, 8 hours and 25 minutes (1,945 minutes in all)"
        );
        assert_eq!(
            result("time between 2026-10-31 09:15Z and 2026-11-01 17:40Z"),
            "1 day, 8 hours and 25 minutes (1,945 minutes in all)"
        );
        assert_eq!(
            result("2026-12-31 minus 2 weeks and 3 days"),
            "2026-12-14 (Monday)"
        );
        assert_eq!(
            result("2026-10-31 minus 3 days and 2 weeks"),
            "2026-10-14 (Wednesday)"
        );
        assert_eq!(
            result("2 weeks and 3 days before 2026-12-31"),
            "2026-12-14 (Monday)"
        );
        assert_eq!(
            result("time between 2026-10-31 9:15 am and 2026-10-31 5:40 pm"),
            "8 hours and 25 minutes (505 minutes in all)"
        );
        assert_eq!(
            result("time between 2026-10-31 at 12:00 am and 2026-10-31 at 12:00 pm"),
            "12 hours (720 minutes in all)"
        );
    }

    #[test]
    fn offsets() {
        assert_eq!(
            result("between 2026-10-31T09:00Z and 2026-10-31T09:00-07:00"),
            "7 hours (420 minutes in all)"
        );
        assert!(error("between 2026-10-31T09:00Z and 2026-10-31 10:00").contains("offset"));
    }

    #[test]
    fn refusals() {
        assert!(error("2026-01-31 plus 1 month").contains("months or years"));
        assert!(error("days until Christmas from today").contains("current date"));
        assert!(error("7/4/2027").contains("2026-10-31"));
        assert!(error("2026-02-30").contains("not a valid date"));
        assert!(error("2026-10-31 plus 3 hours").contains("datetime"));
        assert!(error("how many days").contains("no date"));
        assert!(error("2026-01-01 2026-02-01 2026-03-01").contains("one calculation"));
    }
}
