//! grove.toml: load, validate, explain.
//!
//! - Read `grove/grove.toml` from the committed tree of `~/.workstation` (`git show HEAD:grove/grove.toml`),
//!   never the working tree. `config check --file PATH` reads a draft and says that grove does not use it.
//! - Schema `version = 1`: `[discover] scan` / `ignore` (globs, `~` expanded), `[tags] NAME = [ADDRESS, ...]`,
//!   `[hosts.HOST] production = [ADDRESS, ...]`, `[hosts.HOST.after] ADDRESS = "COMMAND"` (the repositories that
//!   configure HOST and the command that activates each), `[aliases] ADDRESS = [URL, ...]` (other URLs that name
//!   a repository, such as a mirror). Unknown keys are errors that name the key and its line.
//! - Every value keeps its line, so `config check` can say which file and line set it.
//! - Order: TOML syntax, then the secret scan (`secrets`), then the schema. A secret is reported even inside an
//!   unknown key, and its value is never printed.
//! - No committed grove.toml: use the defaults (`scan = ["~/src/*", "~/src/worktrees/*/*"]`) and say so.

mod secrets;

use std::path::{Path, PathBuf};

use serde_json::json;
use toml::Spanned;
use toml::de::{DeTable, DeValue};

use crate::error::{Error, Result};
use crate::git;

pub const DEFAULT_SCAN: &[&str] = &["~/src/*", "~/src/worktrees/*/*"];
const DERIVED_GROUPS: &[&str] = &["behind", "dirty", "unpushed"];
/// Where grove.toml lives inside the config repository.
pub const REPO_PATH: &str = "grove/grove.toml";

/// Where the config came from.
#[derive(Debug, Clone)]
pub enum Origin {
    /// `grove/grove.toml` at HEAD of the config repository.
    Head {
        repo: PathBuf,
        commit: String,
        working_tree_differs: bool,
    },
    /// A file read as written (`config check --file`).
    File(PathBuf),
    /// Nothing committed: the defaults.
    Defaults,
}

/// One value and the line that set it.
#[derive(Debug, Clone)]
pub struct Entry {
    pub key: String,
    pub value: serde_json::Value,
    pub line: usize,
}

/// A list of addresses under a name (`tags.NAME`, `hosts.HOST.production`) and its line.
#[derive(Debug, Clone)]
pub struct Named {
    pub name: String,
    pub addresses: Vec<String>,
    pub line: usize,
}

/// `hosts.HOST.after.ADDRESS = "COMMAND"`: ADDRESS configures HOST, and COMMAND activates it there.
#[derive(Debug, Clone)]
pub struct After {
    pub host: String,
    pub repo: String,
    pub command: String,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub origin: Origin,
    /// The file name used in sources: `grove.toml`, or the `--file` path as given.
    pub label: String,
    pub scan: Vec<String>,
    pub ignore: Vec<String>,
    pub tags: Vec<Named>,
    /// `hosts.HOST.production`, per host.
    pub production: Vec<Named>,
    /// `hosts.HOST.after`, in file order.
    pub after: Vec<After>,
    /// `aliases.ADDRESS`: URLs that also name the repository at ADDRESS.
    pub aliases: Vec<Named>,
    /// Every value, in file order, for `config check`.
    pub entries: Vec<Entry>,
}

impl Config {
    /// What grove uses: `grove/grove.toml` at HEAD of `~/.workstation`, else the defaults.
    pub fn load(home: &Path) -> Result<Config> {
        let repo = home.join(".workstation");
        let head = if repo.is_dir() {
            git::rev_parse(&repo, "HEAD")?
        } else {
            None
        };
        let text = match &head {
            Some(_) => git::show(&repo, "HEAD", REPO_PATH)?,
            None => None,
        };
        let (Some(commit), Some(text)) = (head, text) else {
            return Ok(Config::defaults(Origin::Defaults));
        };
        let working_tree_differs = git::differs_from_head(&repo, REPO_PATH)?;
        let fix = format!("$EDITOR {}", repo.join(REPO_PATH).display());
        let origin = Origin::Head {
            repo,
            commit,
            working_tree_differs,
        };
        parse(&text, "grove.toml", &fix, origin)
    }

    /// A file as written, for `config check --file`.
    pub fn load_file(path: &Path) -> Result<Config> {
        let label = path.display().to_string();
        let text = std::fs::read_to_string(path)
            .map_err(|e| Error::new(format!("cannot read {label}: {e}")).try_(format!("ls -l {label}")))?;
        parse(
            &text,
            &label,
            &format!("$EDITOR {label}"),
            Origin::File(path.to_path_buf()),
        )
    }

    fn defaults(origin: Origin) -> Config {
        Config {
            origin,
            label: "defaults".into(),
            scan: DEFAULT_SCAN.iter().map(ToString::to_string).collect(),
            ignore: Vec::new(),
            tags: Vec::new(),
            production: Vec::new(),
            after: Vec::new(),
            aliases: Vec::new(),
            entries: Vec::new(),
        }
    }

    /// The source of a value set on `line`: `grove.toml:8`.
    pub fn source(&self, line: usize) -> String {
        format!("{}:{line}", self.label)
    }

    /// `hosts.HOST.after` for `host`: its config repositories and their commands.
    pub fn after_on(&self, host: &str) -> Vec<&After> {
        self.after.iter().filter(|a| a.host == host).collect()
    }

    /// The production list for `host`, if the config has one.
    pub fn production_on(&self, host: &str) -> Option<&Named> {
        self.production.iter().find(|p| p.name == host)
    }
}

fn line_of(text: &str, offset: usize) -> usize {
    text[..offset.min(text.len())].matches('\n').count() + 1
}

fn parse(text: &str, label: &str, fix: &str, origin: Origin) -> Result<Config> {
    let root = DeTable::parse(text).map_err(|e| {
        let line = e.span().map_or(1, |s| line_of(text, s.start));
        Error::new(format!("{label}:{line}: {}", e.message().trim_end())).fix(fix)
    })?;
    let root = root.get_ref();

    let mut findings = Vec::new();
    scan_secrets(text, root, "", &mut findings);
    if !findings.is_empty() {
        findings.sort_by_key(|(line, _)| *line);
        let details = findings
            .into_iter()
            .map(|(line, what)| format!("{label}:{line}: {what}"))
            .collect();
        return Err(Error::new(format!(
            "{label} looks like it contains secrets (values not shown)"
        ))
        .details(details)
        .fix(format!("{fix}, then rotate any secret that was ever committed")));
    }

    let mut schema = Schema {
        text,
        config: Config::defaults(origin),
        problems: Vec::new(),
    };
    schema.config.label = label.to_string();
    schema.root(root);
    let Schema {
        mut config,
        mut problems,
        ..
    } = schema;
    match problems.len() {
        0 => {
            config.entries.sort_by_key(|e| e.line);
            Ok(config)
        }
        1 => {
            let (line, what) = problems.remove(0);
            Err(Error::new(format!("{label}:{line}: {what}")).fix(fix))
        }
        n => {
            problems.sort_by_key(|(line, _)| *line);
            let details = problems
                .into_iter()
                .map(|(line, what)| format!("{label}:{line}: {what}"))
                .collect();
            Err(Error::new(format!("{label} has {n} problems"))
                .details(details)
                .fix(fix))
        }
    }
}

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}

/// Every key and string value, depth first. Findings are `(line, "KEY.PATH: rule")`.
fn scan_secrets(text: &str, table: &DeTable<'_>, path: &str, out: &mut Vec<(usize, String)>) {
    for (key, value) in table {
        let path = join(path, key.get_ref());
        if let Some(rule) = secrets::key(key.get_ref()) {
            out.push((line_of(text, key.span().start), format!("{path}: {rule}")));
        } else {
            scan_value(text, value, &path, out);
        }
    }
}

fn scan_value(text: &str, value: &Spanned<DeValue<'_>>, path: &str, out: &mut Vec<(usize, String)>) {
    match value.get_ref() {
        DeValue::String(s) => {
            if let Some(rule) = secrets::value(s) {
                out.push((line_of(text, value.span().start), format!("{path}: {rule}")));
            }
        }
        DeValue::Array(items) => {
            for (i, item) in items.iter().enumerate() {
                scan_value(text, item, &format!("{path}[{i}]"), out);
            }
        }
        DeValue::Table(table) => scan_secrets(text, table, path, out),
        _ => {}
    }
}

/// The schema walk: fills `config` and collects `(line, problem)`.
struct Schema<'t> {
    text: &'t str,
    config: Config,
    problems: Vec<(usize, String)>,
}

type Item<'a, 'i> = (&'a Spanned<std::borrow::Cow<'i, str>>, &'a Spanned<DeValue<'i>>);

/// A table's entries in file order (the parser's map is sorted by key).
fn in_order<'a, 'i>(table: &'a DeTable<'i>) -> Vec<Item<'a, 'i>> {
    let mut items: Vec<_> = table.iter().collect();
    items.sort_by_key(|(k, _)| k.span().start);
    items
}

impl Schema<'_> {
    fn line(&self, span: &std::ops::Range<usize>) -> usize {
        line_of(self.text, span.start)
    }

    fn problem(&mut self, span: &std::ops::Range<usize>, what: String) {
        let line = self.line(span);
        self.problems.push((line, what));
    }

    fn unknown(&mut self, span: &std::ops::Range<usize>, path: &str, key: &str, known: &[&str]) {
        let full = join(path, key);
        let hint = known
            .iter()
            .filter(|k| edit_distance(k, key) <= 2)
            .min_by_key(|k| edit_distance(k, key))
            .map(|k| format!(" (did you mean {}?)", join(path, k)))
            .unwrap_or_default();
        self.problem(span, format!("unknown key {full}{hint}"));
    }

    fn table<'a, 'i>(&mut self, value: &'a Spanned<DeValue<'i>>, key: &str) -> Option<&'a DeTable<'i>> {
        if let DeValue::Table(t) = value.get_ref() {
            Some(t)
        } else {
            self.problem(&value.span(), format!("{key} must be a table"));
            None
        }
    }

    fn strings(&mut self, value: &Spanned<DeValue<'_>>, key: &str) -> Option<Vec<String>> {
        let strings = match value.get_ref() {
            DeValue::Array(items) => items
                .iter()
                .map(|i| i.get_ref().as_str().map(ToString::to_string))
                .collect(),
            _ => None,
        };
        if strings.is_none() {
            self.problem(&value.span(), format!("{key} must be a list of strings"));
        }
        strings
    }

    fn entry(&mut self, key: String, value: serde_json::Value, line: usize) {
        self.config.entries.push(Entry { key, value, line });
    }

    fn root(&mut self, root: &DeTable<'_>) {
        let mut version = false;
        for (key, value) in in_order(root) {
            match key.get_ref().as_ref() {
                "version" => {
                    version = true;
                    let one = matches!(value.get_ref(), DeValue::Integer(i) if i.as_str() == "1");
                    if one {
                        let line = self.line(&key.span());
                        self.entry("version".into(), json!(1), line);
                    } else {
                        self.problem(&value.span(), "version must be 1".into());
                    }
                }
                "discover" => {
                    let Some(table) = self.table(value, "discover") else {
                        continue;
                    };
                    for (key, value) in in_order(table) {
                        let line = self.line(&key.span());
                        let name = key.get_ref().as_ref();
                        if !matches!(name, "scan" | "ignore") {
                            self.unknown(&key.span(), "discover", name, &["scan", "ignore"]);
                            continue;
                        }
                        let path = join("discover", name);
                        let Some(globs) = self.strings(value, &path) else {
                            continue;
                        };
                        self.entry(path, json!(globs), line);
                        if name == "scan" {
                            self.config.scan = globs;
                        } else {
                            self.config.ignore = globs;
                        }
                    }
                }
                "tags" => {
                    let Some(table) = self.table(value, "tags") else {
                        continue;
                    };
                    for (key, value) in in_order(table) {
                        let line = self.line(&key.span());
                        let name = key.get_ref().to_string();
                        let path = join("tags", &name);
                        if DERIVED_GROUPS.contains(&name.as_str()) {
                            self.problem(&key.span(), format!("{path}: @{name} is a derived group"));
                            continue;
                        }
                        let Some(addresses) = self.strings(value, &path) else {
                            continue;
                        };
                        self.entry(path, json!(addresses), line);
                        self.config.tags.push(Named {
                            name,
                            addresses,
                            line,
                        });
                    }
                }
                "hosts" => self.hosts(value),
                "aliases" => self.aliases(value),
                other => self.unknown(
                    &key.span(),
                    "",
                    other,
                    &["version", "discover", "tags", "hosts", "aliases"],
                ),
            }
        }
        if !version {
            self.problems
                .push((1, "version is missing (add version = 1)".into()));
        }
    }
}

impl Schema<'_> {
    /// `hosts`: per host, `production` and `after`.
    fn hosts(&mut self, value: &Spanned<DeValue<'_>>) {
        let Some(hosts) = self.table(value, "hosts") else {
            return;
        };
        for (host, value) in in_order(hosts) {
            let host = host.get_ref().to_string();
            let path = join("hosts", &host);
            let Some(table) = self.table(value, &path) else {
                continue;
            };
            for (key, value) in in_order(table) {
                let line = self.line(&key.span());
                if key.get_ref() == "after" {
                    self.after(&host, value);
                    continue;
                }
                if key.get_ref() != "production" {
                    self.unknown(&key.span(), &path, key.get_ref(), &["production", "after"]);
                    continue;
                }
                let path = join(&path, "production");
                let Some(addresses) = self.strings(value, &path) else {
                    continue;
                };
                self.entry(path, json!(addresses), line);
                self.config.production.push(Named {
                    name: host.clone(),
                    addresses,
                    line,
                });
            }
        }
    }

    /// `aliases`: a table of `ADDRESS = [URL, ...]`.
    fn aliases(&mut self, value: &Spanned<DeValue<'_>>) {
        let Some(table) = self.table(value, "aliases") else {
            return;
        };
        for (key, value) in in_order(table) {
            let line = self.line(&key.span());
            let name = key.get_ref().to_string();
            let path = join("aliases", &name);
            let Some(urls) = self.strings(value, &path) else {
                continue;
            };
            self.entry(path, json!(urls), line);
            self.config.aliases.push(Named {
                name,
                addresses: urls,
                line,
            });
        }
    }

    /// `hosts.HOST.after`: a table of `ADDRESS = "COMMAND"`.
    fn after(&mut self, host: &str, value: &Spanned<DeValue<'_>>) {
        let path = format!("hosts.{host}.after");
        let Some(table) = self.table(value, &path) else {
            return;
        };
        for (key, value) in in_order(table) {
            let line = self.line(&key.span());
            let repo = key.get_ref().to_string();
            let path = join(&path, &repo);
            let Some(command) = value.get_ref().as_str().map(ToString::to_string) else {
                self.problem(&value.span(), format!("{path} must be a string (a command)"));
                continue;
            };
            self.entry(path, json!(command), line);
            self.config.after.push(After {
                host: host.to_string(),
                repo,
                command,
            });
        }
    }
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut prev = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let next = (prev + usize::from(ca != *cb))
                .min(row[j] + 1)
                .min(row[j + 1] + 1);
            prev = row[j + 1];
            row[j + 1] = next;
        }
    }
    row[b.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(text: &str) -> Result<Config> {
        parse(text, "t.toml", "$EDITOR t.toml", Origin::File("t.toml".into()))
    }

    #[test]
    fn values_keep_their_lines_in_file_order() {
        let config = check("version = 1\n[tags]\nz = [\"a\"]\nb = [\"c\"]\n").unwrap();
        let keys: Vec<_> = config.entries.iter().map(|e| (e.key.as_str(), e.line)).collect();
        assert_eq!(keys, [("version", 1), ("tags.z", 3), ("tags.b", 4)]);
        assert_eq!(config.scan, DEFAULT_SCAN);
    }

    #[test]
    fn problems_name_the_line() {
        let err = check("version = 2\n").unwrap_err();
        assert_eq!(err.message, "t.toml:1: version must be 1");
        let err = check("version = 1\n[tags]\ndirty = [\"a\"]\n").unwrap_err();
        assert_eq!(err.message, "t.toml:3: tags.dirty: @dirty is a derived group");
        let err = check("version = 1\n[hosts.h]\nprodution = [\"a\"]\n").unwrap_err();
        assert_eq!(
            err.message,
            "t.toml:3: unknown key hosts.h.prodution (did you mean hosts.h.production?)"
        );
    }

    #[test]
    fn hosts_after_and_aliases() {
        let config = check(
            "version = 1\n[hosts.h.after]\n\".workstation\" = \"rebuild\"\n[aliases]\nkit = [\"https://m.example/kit\"]\n",
        )
        .unwrap();
        let after = config.after_on("h");
        assert_eq!(
            (after[0].repo.as_str(), after[0].command.as_str()),
            (".workstation", "rebuild")
        );
        assert_eq!(config.aliases[0].addresses, ["https://m.example/kit"]);
        let err = check("version = 1\n[hosts.h.after]\nx = 1\n").unwrap_err();
        assert_eq!(
            err.message,
            "t.toml:3: hosts.h.after.x must be a string (a command)"
        );
        let err = check("version = 1\n[hosts.h]\naftr = {}\n").unwrap_err();
        assert_eq!(
            err.message,
            "t.toml:3: unknown key hosts.h.aftr (did you mean hosts.h.after?)"
        );
    }

    #[test]
    fn edit_distances() {
        assert_eq!(edit_distance("scan", "scna"), 2);
        assert_eq!(edit_distance("tags", "tag"), 1);
        assert_eq!(edit_distance("", "abc"), 3);
    }
}
