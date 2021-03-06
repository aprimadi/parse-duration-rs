# parse-duration-rs

parse-duration-rs is a Rust port of Golang parse duration `time.ParseDuration`.
It parses a duration string in a short form such as `100ms`, `1h45m`, and `3ns`
and return duration in nanoseconds.

The crate is called `go-parse-duration` and you can depend on it via cargo:

```ini
[dependencies]
go-parse-duration = "0.1"
```

## Example

```rust
use go_parse_duration::{parse_duration, Error};

fn parse() -> Result<i64, Error> {
  let d = parse_duration("300us")?;
  Ok(d)
}
```

**Usage with Chrono**

Converting to Chrono duration can be done easily:

```rust
use chrono::Duration;
use go_parse_duration::{parse_duration, Error};

fn parse() -> Result<Duration, Error> {
  let d = parse_duration("1m")?;
  Ok(Duration::nanoseconds(d))
}
```

## Author

Armin Primadi https://github.com/aprimadi (@ [Sahamee](https://www.sahamee.com))

