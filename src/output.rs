//! Rendering: human tables and the `grove/1` JSON envelope.
//!
//! - Human: columns padded by character count and separated by two spaces, no trailing whitespace, `~` for the
//!   home directory, and a summary line. G0 prints no color.
//! - JSON: pretty-printed, `{"schema": "grove/1", "command": ...}` first. Absolute paths.

use std::time::{Duration, SystemTime};

use serde::Serialize;

use crate::error::Error;

pub const SCHEMA: &str = "grove/1";

#[derive(Serialize)]
struct Envelope<'a, T> {
    schema: &'static str,
    command: &'a str,
    #[serde(flatten)]
    body: T,
}

/// Print one `grove/1` JSON document on stdout.
pub fn print_json(command: &str, body: impl Serialize) {
    let doc = Envelope {
        schema: SCHEMA,
        command,
        body,
    };
    let text = serde_json::to_string_pretty(&doc).expect("grove's JSON models always serialize");
    println!("{text}");
}

/// Show an error: JSON on stdout with `--json`, else the human form on stderr.
pub fn print_error(command: &str, json: bool, error: &Error) {
    if json {
        #[derive(Serialize)]
        struct Body<'a> {
            error: &'a Error,
        }
        print_json(command, Body { error });
    } else {
        eprintln!("{error}");
    }
}

/// A table of left-aligned columns.
#[derive(Default)]
pub struct Table {
    rows: Vec<Vec<String>>,
}

impl Table {
    pub fn new<S: Into<String>>(header: impl IntoIterator<Item = S>) -> Self {
        let mut table = Table::default();
        table.row(header);
        table
    }

    pub fn row<S: Into<String>>(&mut self, cells: impl IntoIterator<Item = S>) {
        self.rows.push(cells.into_iter().map(Into::into).collect());
    }

    /// The rows, each prefixed with `indent`, without trailing whitespace, each ending in a newline.
    pub fn render(&self, indent: &str) -> String {
        let columns = self.rows.iter().map(Vec::len).max().unwrap_or(0);
        let widths: Vec<usize> = (0..columns)
            .map(|c| {
                self.rows
                    .iter()
                    .filter_map(|r| r.get(c))
                    .map(|s| s.chars().count())
                    .max()
                    .unwrap_or(0)
            })
            .collect();
        let mut out = String::new();
        for row in &self.rows {
            let mut line = String::from(indent);
            for (cell, width) in row.iter().zip(&widths) {
                line.push_str(cell);
                line.extend(std::iter::repeat_n(' ', width - cell.chars().count() + 2));
            }
            out.push_str(line.trim_end());
            out.push('\n');
        }
        out
    }
}

/// RFC 3339 in UTC, to the second: `2026-11-01T12:00:00Z`. A time before 1970 prints as 1970.
pub fn timestamp(time: SystemTime) -> String {
    let secs = time
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    let (days, rest) = (secs / 86_400, secs % 86_400);
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest / 60 % 60,
        rest % 60
    )
}

/// The inverse of [`timestamp`]: exactly `YYYY-MM-DDTHH:MM:SSZ`, as grove writes it, else `None`.
pub fn parse_timestamp(text: &str) -> Option<SystemTime> {
    let b = text.as_bytes();
    if b.len() != 20 || [b[4], b[7], b[10], b[13], b[16], b[19]] != *b"--T::Z" {
        return None;
    }
    let num = |range: std::ops::Range<usize>| -> Option<u64> {
        let digits = text.get(range)?;
        digits
            .bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| digits.parse().ok())?
    };
    let (year, month, day) = (num(0..4)?, num(5..7)?, num(8..10)?);
    let (hour, minute, second) = (num(11..13)?, num(14..16)?, num(17..19)?);
    if year < 1970 || !(1..=12).contains(&month) || day == 0 || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let days = days_from_civil(year, month, day);
    if civil_from_days(days) != (year, month, day) {
        return None; // a day past the month's end
    }
    let secs = days * 86_400 + hour * 3600 + minute * 60 + second;
    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(secs))
}

// Days since 1970-01-01 and the proleptic Gregorian calendar, after Howard Hinnant's `days_from_civil` and
// `civil_from_days` (http://howardhinnant.github.io/date_algorithms.html), for dates from 1970 on.

fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + u64::from(month <= 2);
    (year, month, day)
}

fn days_from_civil(year: u64, month: u64, day: u64) -> u64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year / 400;
    let yoe = year % 400;
    let mp = if month > 2 { month - 3 } else { month + 9 };
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// How long ago `then` was, as of `now`: `12s ago`, `5m ago`, `3h ago`, `2d ago`.
pub fn age(now: SystemTime, then: SystemTime) -> String {
    let secs = now.duration_since(then).unwrap_or(Duration::ZERO).as_secs();
    match secs {
        0..60 => format!("{secs}s ago"),
        60..3600 => format!("{}m ago", secs / 60),
        3600..86_400 => format!("{}h ago", secs / 3600),
        _ => format!("{}d ago", secs / 86_400),
    }
}

/// `1 repo`, `2 repos`.
pub fn plural(n: usize, noun: &str) -> String {
    if n == 1 {
        format!("{n} {noun}")
    } else {
        format!("{n} {noun}s")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_pads_by_characters_and_trims() {
        let mut table = Table::new(["A", "B", "C"]);
        table.row(["↑2", "x", ""]);
        assert_eq!(table.render(""), "A   B  C\n↑2  x\n");
    }

    #[test]
    fn timestamps() {
        let at = |secs| SystemTime::UNIX_EPOCH + Duration::from_secs(secs);
        let cases = [
            (0, "1970-01-01T00:00:00Z"),
            (951_782_400, "2000-02-29T00:00:00Z"),
            (1_793_534_400, "2026-11-01T12:00:00Z"),
            (4_107_542_399, "2100-02-28T23:59:59Z"),
        ];
        for (secs, text) in cases {
            assert_eq!(timestamp(at(secs)), text);
            assert_eq!(parse_timestamp(text), Some(at(secs)));
        }
        assert_eq!(
            timestamp(at(5) + Duration::from_millis(900)),
            "1970-01-01T00:00:05Z"
        );
        for bad in [
            "",
            "2026-11-01",
            "2026-11-01T12:00:00",
            "2026-13-01T12:00:00Z",
            "2026-02-30T00:00:00Z",
            "x026-11-01T12:00:00Z",
        ] {
            assert_eq!(parse_timestamp(bad), None, "{bad}");
        }
    }

    #[test]
    fn ages() {
        let t = SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let ago = |s| age(t, t - Duration::from_secs(s));
        assert_eq!(
            [ago(0), ago(59), ago(60), ago(3599), ago(3600), ago(86_400 * 3)],
            ["0s ago", "59s ago", "1m ago", "59m ago", "1h ago", "3d ago"]
        );
        assert_eq!(age(t, t + Duration::from_secs(5)), "0s ago");
    }
}
