//! "Looks like a secret": rules applied to every key and string value of grove.toml.
//!
//! - Key names containing `token`, `secret`, `password`, `passwd`, `apikey`/`api_key`, or `private_key`.
//! - URLs with a password in their userinfo (`https://user:pass@host`).
//! - Known credential prefixes and PEM headers (`-----BEGIN`).
//! - Long high-entropy strings that are not paths or globs.
//!
//! Findings name the file, line, key path and the rule, never the value.
