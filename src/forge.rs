//! From a remote URL to a forge page and an `owner/name`.
//!
//! `git@host:owner/name.git`, `ssh://[user@]host[:port]/owner/name.git` and `https://host/owner/name[.git]` all
//! become `https://host/owner/name`. Userinfo is always dropped. A local path or `file://` remote has no forge
//! page. The remote is `origin` if present, else the first remote.
