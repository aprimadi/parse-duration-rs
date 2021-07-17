pub enum Error {
    ParseError(String),
}

enum InternalError {
    Overflow,
}

/// parse_duration parses a duration string and return duration in nanoseconds.
///
/// A duration string is a possibly signed sequence of decimal numbers, each
/// with optional fraction and a unit suffix, such as "300ms", "-1.5h", or 
/// "2h45m".
///
/// Valid time units are "ns", "us" (or "µs"), "ms", "s", "m", "h".
pub fn parse_duration(string: &str) -> Result<i64, Error> {
    /*
	orig := s
	var d int64
	neg := false

	// Consume [-+]?
	if s != "" {
		c := s[0]
		if c == '-' || c == '+' {
			neg = c == '-'
			s = s[1:]
		}
	}
	// Special case: if all that is left is "0", this is zero.
	if s == "0" {
		return 0, nil
	}
	if s == "" {
		return 0, errors.New("time: invalid duration " + quote(orig))
	}
	for s != "" {
		var (
			v, f  int64       // integers before, after decimal point
			scale float64 = 1 // value = v + f/scale
		)

		var err error

		// The next character must be [0-9.]
		if !(s[0] == '.' || '0' <= s[0] && s[0] <= '9') {
			return 0, errors.New("time: invalid duration " + quote(orig))
		}
		// Consume [0-9]*
		pl := len(s)
		v, s, err = leadingInt(s)
		if err != nil {
			return 0, errors.New("time: invalid duration " + quote(orig))
		}
		pre := pl != len(s) // whether we consumed anything before a period

		// Consume (\.[0-9]*)?
		post := false
		if s != "" && s[0] == '.' {
			s = s[1:]
			pl := len(s)
			f, scale, s = leadingFraction(s)
			post = pl != len(s)
		}
		if !pre && !post {
			// no digits (e.g. ".s" or "-.s")
			return 0, errors.New("time: invalid duration " + quote(orig))
		}

		// Consume unit.
		i := 0
		for ; i < len(s); i++ {
			c := s[i]
			if c == '.' || '0' <= c && c <= '9' {
				break
			}
		}
		if i == 0 {
			return 0, errors.New("time: missing unit in duration " + quote(orig))
		}
		u := s[:i]
		s = s[i:]
		unit, ok := unitMap[u]
		if !ok {
			return 0, errors.New("time: unknown unit " + quote(u) + " in duration " + quote(orig))
		}
		if v > (1<<63-1)/unit {
			// overflow
			return 0, errors.New("time: invalid duration " + quote(orig))
		}
		v *= unit
		if f > 0 {
			// float64 is needed to be nanosecond accurate for fractions of hours.
			// v >= 0 && (f*unit/scale) <= 3.6e+12 (ns/h, h is the largest unit)
			v += int64(float64(f) * (float64(unit) / scale))
			if v < 0 {
				// overflow
				return 0, errors.New("time: invalid duration " + quote(orig))
			}
		}
		d += v
		if d < 0 {
			// overflow
			return 0, errors.New("time: invalid duration " + quote(orig))
		}
	}

	if neg {
		d = -d
	}
	return Duration(d), nil
    */
    // [-+]?([0-9]*(\.[0-9]*)?[a-z]+)+
    let mut s = string;
    let mut d: i64 = 0; // duration to be returned
    let mut neg = false;

    // Consume [-+]?
    if s != "" {
        let c = s.chars().nth(0).unwrap();
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
        let mut v: i64 = 0;
        let mut f: i64 = 0;
        // value = v + f / scale
        let mut scale: f64 = 1f64;

        // The next character must be [0-9.]
        let c = s.chars().nth(0).unwrap();
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
                return Err(Error::ParseError(format!("invalid duration: {}", string)));
            }
        }
        let pre = pl != s.len(); // whether we consume anything before a period

		// Consume (\.[0-9]*)?
        let mut post = false;
        if s != "" && s.chars().nth(0).unwrap() == '.' {
            s = &s[1..];
            let pl = s.len();
            match leading_fraction(s) {
                (f_, scale_, s_) => {
                    f = f_;
                    scale = scale_;
                    s = s_;
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
            let c = s.chars().nth(i).unwrap();
            if c == '.' || '0' <= c && c <= '9' {
                break;
            }
            i += 1;
        }
        if i == 0 {
            return Err(Error::ParseError(format!("missing unit in duration: {}", string)));
        }
        let u = &s[..i];
        s = &s[i..];
        let unit = match u {
            "ns" => 1i64,
            "us" => 1000i64,
            "µs" => 1000i64, // U+00B5 = micro symbol
            "μs" => 1000i64, // U+03BC = Greek letter mu
            "ms" => 1000000i64,
            "s" =>  1000000000i64,
            "m" =>  60000000000i64,
            "h" =>  3600000000000i64,
            _ => { 
                return Err(Error::ParseError(format!("unknown unit {} in duration {}", u, string)));
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
            v += i64::from(f64::from(f) * (f64::from(unit) / scale));
        }
        // TODO
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
        let c = s.chars().nth(i).unwrap();
        if c < '0' || c > '9' {
            break
        }
        if x > (1<<63-1)/10 {
            return Err(InternalError::Overflow);
        }
        let d = i64::from(c.to_digit(10).unwrap());
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
fn leading_fraction(s: &str) -> (i64, f64, &str) {
    let mut i = 0;
    let mut x = 0i64;
    let mut scale = 1f64;
    let mut overflow = false;
    while i < s.len() {
        let c = s.chars().nth(i).unwrap();
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
        let d = i64::from(c.to_digit(10).unwrap());
        let y = x * 10 + d;
        if y < 0 {
            overflow = true;
            continue;
        }
        x = y;
        scale *= 10f64;
        i += 1;
    }
    (x, scale, &s[i..])
}

