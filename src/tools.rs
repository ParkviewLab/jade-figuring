// SPDX-FileCopyrightText: 2026 Gary Frattarola <garyf@parkviewlab.ai>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! The two tools. Their definitions are written by hand, so that nothing adds
//! a `$schema`, titles or descriptions: every character of them is sent with
//! every request a client makes to its model.

use serde_json::{Map, Value, json};

pub const ENGINEERING_CALCULATOR: &str = "engineering_calculator";
pub const DATE_CALCULATOR: &str = "date_calculator";

/// Name, description and input schema of each tool.
pub fn definitions() -> Vec<(&'static str, &'static str, Map<String, Value>)> {
    let schema = || match json!({
        "type": "object",
        "properties": {"expression": {"type": "string"}},
        "required": ["expression"]
    }) {
        Value::Object(m) => m,
        _ => unreachable!(),
    };
    vec![
        (
            ENGINEERING_CALCULATOR,
            "Evaluates one expression, written as on an engineering calculator.",
            schema(),
        ),
        (
            DATE_CALCULATOR,
            "Calculates with dates and times: days or time between two, a date plus or minus a period, the weekday of a date.",
            schema(),
        ),
    ]
}

/// The expression from the call's one string argument, whatever it is named.
pub fn expression(arguments: Option<&Map<String, Value>>) -> Result<String, String> {
    let Some(args) = arguments else {
        return Err("no expression was given; pass it as the argument \"expression\"".into());
    };
    if let Some(Value::String(s)) = args.get("expression") {
        return Ok(s.clone());
    }
    let strings: Vec<&String> = args
        .values()
        .filter_map(|v| {
            if let Value::String(s) = v {
                Some(s)
            } else {
                None
            }
        })
        .collect();
    match strings.len() {
        1 => Ok(strings[0].clone()),
        0 => Err("no expression was given; pass it as the argument \"expression\"".into()),
        _ => Err("several text arguments were given; pass one, named \"expression\"".into()),
    }
}

/// Runs a tool: the result text, or the error text.
pub fn run(name: &str, expression: &str) -> Result<String, String> {
    match name {
        ENGINEERING_CALCULATOR => jade_figuring::calc::calculate(expression).map_err(|e| e.message),
        DATE_CALCULATOR => jade_figuring::dates::calculate(expression).map_err(|e| e.message),
        other => Err(format!("there is no tool named '{other}'")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_single_string_argument_is_the_expression() {
        let named = |k: &str| {
            let mut m = Map::new();
            m.insert(k.into(), json!("2+2"));
            m
        };
        assert_eq!(expression(Some(&named("expression"))).unwrap(), "2+2");
        assert_eq!(expression(Some(&named("query"))).unwrap(), "2+2");
        assert!(
            expression(Some(&Map::new()))
                .unwrap_err()
                .contains("no expression")
        );
        assert!(expression(None).unwrap_err().contains("no expression"));
        let mut two = named("a");
        two.insert("b".into(), json!("3"));
        assert!(expression(Some(&two)).unwrap_err().contains("several"));
    }

    #[test]
    fn both_tools_run() {
        assert_eq!(run(ENGINEERING_CALCULATOR, "2^10").unwrap(), "2^10 = 1024");
        assert_eq!(
            run(DATE_CALCULATOR, "weekday of 2027-07-04").unwrap(),
            "weekday of 2027-07-04 = Sunday"
        );
        assert!(run("other", "1").is_err());
    }
}
