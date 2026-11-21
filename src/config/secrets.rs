//! "Looks like a secret": rules applied to every key and string value of grove.toml.
//!
//! - Key names containing `token`, `secret`, `password`, `passwd`, `apikey`/`api_key`, or `private_key`.
//! - URLs with a password in their userinfo (`https://user:pass@host`).
//! - Known credential prefixes and PEM headers (`-----BEGIN`).
//! - Long high-entropy strings that are not paths or globs.
//!
//! Findings name the key path and the rule, never the value.

const KEY_WORDS: &[&str] = &[
    "token",
    "secret",
    "password",
    "passwd",
    "apikey",
    "api_key",
    "private_key",
];

/// Prefixes of well-known credential formats (GitHub, GitLab, Slack, AWS, OpenAI-style, Anthropic).
const PREFIXES: &[&str] = &[
    "ghp_",
    "gho_",
    "ghu_",
    "ghs_",
    "ghr_",
    "github_pat_",
    "glpat-",
    "xoxb-",
    "xoxp-",
    "AKIA",
    "sk-",
];

/// Why a key name looks like it holds a secret.
pub fn key(name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    KEY_WORDS
        .iter()
        .find(|w| lower.contains(*w))
        .map(|w| format!("the key name contains \"{w}\""))
}

/// Why a string value looks like a secret.
pub fn value(text: &str) -> Option<&'static str> {
    if text.contains("-----BEGIN") {
        return Some("a PEM header");
    }
    if url_has_password(text) {
        return Some("a URL with a password");
    }
    if PREFIXES
        .iter()
        .any(|p| text.starts_with(p) && text.len() >= p.len() + 16)
    {
        return Some("a known credential prefix");
    }
    if random_looking(text) {
        return Some("a long random-looking string");
    }
    None
}

fn url_has_password(text: &str) -> bool {
    let Some((_, rest)) = text.split_once("://") else {
        return false;
    };
    let authority = rest.split('/').next().unwrap_or(rest);
    authority
        .rsplit_once('@')
        .is_some_and(|(userinfo, _)| userinfo.contains(':'))
}

/// At least 32 characters, no path or glob characters or spaces, and high entropy.
fn random_looking(text: &str) -> bool {
    if text.len() < 32 || text.contains(['/', '~', '*', ' ', '.']) {
        return false;
    }
    let mut counts = [0u32; 256];
    for b in text.bytes() {
        counts[usize::from(b)] += 1;
    }
    #[allow(clippy::cast_precision_loss)] // lengths of config strings
    let len = text.len() as f64;
    let entropy: f64 = counts
        .iter()
        .filter(|&&c| c > 0)
        .map(|&c| {
            let p = f64::from(c) / len;
            -p * p.log2()
        })
        .sum();
    entropy > 4.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_names() {
        assert_eq!(
            key("api_token").as_deref(),
            Some("the key name contains \"token\"")
        );
        assert_eq!(
            key("GITHUB_PASSWORD").as_deref(),
            Some("the key name contains \"password\"")
        );
        assert_eq!(key("production"), None);
        assert_eq!(key("tools"), None);
    }

    #[test]
    fn values() {
        assert_eq!(
            value("https://daniel:pw@git.example.org/x.git"),
            Some("a URL with a password")
        );
        assert_eq!(value("-----BEGIN OPENSSH PRIVATE KEY-----"), Some("a PEM header"));
        assert_eq!(
            value("ghp_0123456789abcdefABCDEF0123"),
            Some("a known credential prefix")
        );
        assert_eq!(
            value("q8Zt3LxV0pWm9RkC2yHn7BdF4sGj6UeA"),
            Some("a long random-looking string")
        );
    }

    #[test]
    fn ordinary_values_pass() {
        for ok in [
            "https://daniel@git.example.org/x.git",
            "git@github.com:daniel/grove.git",
            "~/src/worktrees/*/*",
            "/home/daniel/src/a-very-long-directory-name-for-testing",
            "alpha",
            "skip-this-one",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ] {
            assert_eq!(value(ok), None, "{ok}");
        }
    }
}
