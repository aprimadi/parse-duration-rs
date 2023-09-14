//!
//! parse-duration-rs is a Rust port of Golang parse duration `time.ParseDuration`.
//! It parses a duration string in a short form such as `100ms`, `1h45m`, and `3ns`
//! and return duration in nanoseconds.
//!
//! The crate is called `go-parse-duration` and you can depend on it via cargo:
//!
//! ```ini
//! [dependencies]
//! go-parse-duration = "0.1"
//! ```
//!
//! ## Example
//!
//! ```rust
//! use go_parse_duration::{parse_duration, Error};
//!
//! fn parse() -> Result<i64, Error> {
//!   let d = parse_duration("300us")?;
//!   Ok(d)
//! }
//! ```
//!
//! **Usage with Chrono**
//!
//! Converting to Chrono duration can be done easily:
//!
//! ```rust
//! use chrono::Duration;
//! use go_parse_duration::{parse_duration, Error};
//!
//! fn parse() -> Result<Duration, Error> {
//!   let d = parse_duration("1m")?;
//!   Ok(Duration::nanoseconds(d))
//! }
//! ```
//!
use std::fmt;

#[derive(Debug, PartialEq)]
pub enum Error {
    ParseError(String),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Error::ParseError(message) = self;
        write!(formatter, "Parse error: {}", message)
    }
}

enum InternalError {
    Overflow,
    NaC,
    NaN,
}

/// parse_duration parses a duration string and return duration in nanoseconds.
///
/// A duration string is a possibly signed sequence of decimal numbers, each
/// with optional fraction and a unit suffix, such as "300ms", "-1.5h", or
/// "2h45m".
///
/// Valid time units are "ns", "us" (or "µs"), "ms", "s", "m", "h".
pub fn parse_duration(string: &str) -> Result<i64, Error> {
    // [-+]?([0-9]*(\.[0-9]*)?[a-z]+)+
    let mut s = string;
    let mut d: i64 = 0; // duration to be returned
    let mut neg = false;

    // Consume [-+]?

    if s != "" {
        let Some(c) = s.chars().nth(0) else {
            // error message here
            return Err(Error::ParseError(format!("invalid duration: {}", string)));
        };
        if c == '-' || c == '+' {
            neg = c == '-';
            s = &s[1..];
        }
    }
    // Special case: if all that is left is "0", this is zero.
    if s == "0" {
        return Ok(0);
    }
    if s == "" {
        return Err(Error::ParseError(format!("invalid duration: {}", string)));
    }
    while s != "" {
        // integers before, after decimal point
        let mut v: i64;
        let mut f: i64 = 0;
        // value = v + f / scale
        let mut scale: f64 = 1f64;

        // The next character must be [0-9.]
        let Some(c) = s.chars().nth(0) else {
            // error message here
            return Err(Error::ParseError(format!("invalid duration: {}", string)));
        };
        if !(c == '.' || '0' <= c && c <= '9') {
            return Err(Error::ParseError(format!("invalid duration: {}", string)));
        }

        // Consume [0-9]*
        let pl = s.len();
        match leading_int(s) {
            Ok((_v, _s)) => {
                v = _v;
                s = _s;
            }
            Err(_) => {
                return Err(Error::ParseError(format!(
                    "invalid character in: {}",
                    string
                )));
            }
        }
        let pre = pl != s.len(); // whether we consume anything before a period

        // Consume (\.[0-9]*)?
        let mut post = false;

        if s != "" && s.chars().nth(0) == Some('.') {
            s = &s[1..];
            let pl = s.len();
            match leading_fraction(s) {
                Ok((f_, scale_, s_)) => {
                    f = f_;
                    scale = scale_;
                    s = s_;
                }
                Err(_) => {
                    return Err(Error::ParseError(format!(
                        "invalid character in: {}",
                        string
                    )));
                }
            }
            post = pl != s.len();
        }
        if !pre && !post {
            // no digits (e.g. ".s" or "-.s")
            return Err(Error::ParseError(format!("invalid duration: {}", string)));
        }

        // Consume unit.
        let mut i = 0;
        while i < s.len() {
            let Some(c) = s.chars().nth(i) else {
                // error message here
                return Err(Error::ParseError(format!("invalid duration: {}", string)));
            };
            if c == '.' || '0' <= c && c <= '9' {
                break;
            }
            i += 1;
        }
        if i == 0 {
            return Err(Error::ParseError(format!(
                "missing unit in duration: {}",
                string
            )));
        }
        let u = &s[..i];
        s = &s[i..];
        let unit = match u {
            "ns" => 1i64,
            "us" => 1000i64,
            "µs" => 1000i64, // U+00B5 = micro symbol
            "μs" => 1000i64, // U+03BC = Greek letter mu
            "ms" => 1000000i64,
            "s" => 1000000000i64,
            "m" => 60000000000i64,
            "h" => 3600000000000i64,
            _ => {
                return Err(Error::ParseError(format!(
                    "unknown unit {} in duration {}",
                    u, string
                )));
            }
        };
        if v > (1 << 63 - 1) / unit {
            // overflow
            return Err(Error::ParseError(format!("invalid duration {}", string)));
        }
        v *= unit;
        if f > 0 {
            // f64 is needed to be nanosecond accurate for fractions of hours.
            // v >= 0 && (f*unit/scale) <= 3.6e+12 (ns/h, h is the largest unit)
            v += (f as f64 * (unit as f64 / scale)) as i64;
            if v < 0 {
                // overflow
                return Err(Error::ParseError(format!("invalid duration {}", string)));
            }
        }
        d += v;
        if d < 0 {
            // overflow
            return Err(Error::ParseError(format!("invalid duration {}", string)));
        }
    }
    if neg {
        d = -d;
    }
    Ok(d)
}

// leading_int consumes the leading [0-9]* from s.
fn leading_int(s: &str) -> Result<(i64, &str), InternalError> {
    let mut x = 0;
    let mut i = 0;
    while i < s.len() {
        let Some(c) = s.chars().nth(i) else {
            return Err(InternalError::NaC);
        };
        if c < '0' || c > '9' {
            break;
        }
        if x > (1 << 63 - 1) / 10 {
            return Err(InternalError::Overflow);
        }

        let Some(f) = c.to_digit(10) else {
            return Err(InternalError::NaN)
        };

        let d = i64::from(f);
        x = x * 10 + d;
        if x < 0 {
            // overflow
            return Err(InternalError::Overflow);
        }
        i += 1;
    }
    Ok((x, &s[i..]))
}

// leading_fraction consumes the leading [0-9]* from s.
//
// It is used only for fractions, so does not return an error on overflow,
// it just stops accumulating precision.
//
// It returns (value, scale, remainder) tuple.
fn leading_fraction(s: &str) -> Result<(i64, f64, &str), InternalError> {
    let mut i = 0;
    let mut x = 0i64;
    let mut scale = 1f64;
    let mut overflow = false;
    while i < s.len() {
        let Some(c) = s.chars().nth(i) else {
            return Err(InternalError::NaC);
        };

        if c < '0' || c > '9' {
            break;
        }
        if overflow {
            continue;
        }
        if x > (1 << 63 - 1) / 10 {
            // It's possible for overflow to give a positive number, so take care.
            overflow = true;
            continue;
        }

        let Some(f) = c.to_digit(10) else {
            // error message here
            break;
        };

        let d = i64::from(f);
        let y = x * 10 + d;
        if y < 0 {
            overflow = true;
            continue;
        }
        x = y;
        scale *= 10f64;
        i += 1;
    }
    Ok((x, scale, &s[i..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_duration() -> Result<(), Error> {
        assert_eq!(parse_duration("50ns")?, 50);
        assert_eq!(parse_duration("3ms")?, 3000000);
        assert_eq!(parse_duration("2us")?, 2000);
        assert_eq!(parse_duration("4s")?, 4000000000);
        assert_eq!(parse_duration("1h45m")?, 6300000000000);
        assert_eq!(
            parse_duration("1").unwrap_err(),
            Error::ParseError(String::from("missing unit in duration: 1")),
        );
        assert_eq!(parse_duration("-1h45m")?, -6300000000000);
        assert_eq!(parse_duration("+1h45m")?, 6300000000000);
        assert_eq!(
            parse_duration("a1ns").unwrap_err(),
            Error::ParseError(String::from("invalid duration: a1ns"))
        );
        assert_eq!(
            parse_duration("++50ns").unwrap_err(),
            Error::ParseError(String::from("invalid duration: ++50ns"))
        );
        assert_eq!(
            parse_duration("+").unwrap_err(),
            Error::ParseError(String::from("invalid duration: +"))
        );
        Ok(())
    }
}
