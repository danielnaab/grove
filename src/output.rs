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
    let doc = Envelope { schema: SCHEMA, command, body };
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
            .map(|c| self.rows.iter().filter_map(|r| r.get(c)).map(|s| s.chars().count()).max().unwrap_or(0))
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

/// RFC 3339 in UTC, to the second.
pub fn timestamp(time: SystemTime) -> String {
    humantime::format_rfc3339_seconds(time).to_string()
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
    if n == 1 { format!("{n} {noun}") } else { format!("{n} {noun}s") }
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
