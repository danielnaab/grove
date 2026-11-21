//! From a remote URL to a forge page and an `owner/name`.
//!
//! `git@host:owner/name.git`, `ssh://[user@]host[:port]/owner/name.git`, `git://host/owner/name.git` and
//! `https://[user[:pass]@]host[:port]/owner/name[.git]` all become `https://host[:port]/owner/name` (an SSH port
//! is dropped, an HTTPS port kept). Userinfo is always dropped. A local path or `file://` remote has no forge page.

use crate::git::Remote;

/// A forge page: `https://host/owner/name`, and the `owner/name` part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Forge {
    pub url: String,
    pub slug: String,
}

/// The remote whose forge page grove shows: `origin` if present, else the first.
pub fn primary(remotes: &[Remote]) -> Option<&Remote> {
    remotes
        .iter()
        .find(|r| r.name == "origin")
        .or_else(|| remotes.first())
}

pub fn parse(url: &str) -> Option<Forge> {
    let (host, path) = if let Some((scheme, rest)) = url.split_once("://") {
        let (authority, path) = rest.split_once('/')?;
        let host = authority.rsplit_once('@').map_or(authority, |(_, h)| h);
        let host = match scheme {
            "https" | "http" => host,
            "ssh" | "git" | "git+ssh" | "ssh+git" => host.split(':').next()?,
            _ => return None,
        };
        (host, path)
    } else {
        // scp-like: [user@]host:path. A colon after a slash is a local path, not a host.
        let (left, path) = url.split_once(':')?;
        if left.contains('/') || left.is_empty() || path.starts_with("//") {
            return None;
        }
        (left.rsplit_once('@').map_or(left, |(_, h)| h), path)
    };
    let slug = path
        .trim_matches('/')
        .trim_end_matches(".git")
        .trim_end_matches('/');
    if host.is_empty() || slug.is_empty() || !slug.contains('/') {
        return None;
    }
    Some(Forge {
        url: format!("https://{host}/{slug}"),
        slug: slug.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page(url: &str) -> Option<String> {
        parse(url).map(|f| f.url)
    }

    #[test]
    fn forge_urls() {
        let want = Some("https://git.example.org/daniel/beta".to_string());
        for url in [
            "git@git.example.org:daniel/beta.git",
            "ssh://git@git.example.org:2222/daniel/beta.git",
            "ssh://git.example.org/daniel/beta",
            "https://git.example.org/daniel/beta.git",
            "https://daniel:hunter2@git.example.org/daniel/beta.git",
            "https://git.example.org/daniel/beta/",
            "git://git.example.org/daniel/beta.git",
        ] {
            assert_eq!(page(url), want, "{url}");
        }
        assert_eq!(
            page("https://host:8443/a/b.git").as_deref(),
            Some("https://host:8443/a/b")
        );
        assert_eq!(
            parse("git@host:group/sub/name.git").unwrap().slug,
            "group/sub/name"
        );
    }

    #[test]
    fn local_remotes_have_no_forge_page() {
        for url in [
            "/srv/git/beta.git",
            "../beta",
            "file:///srv/git/beta.git",
            "./x:y/z",
            "beta",
        ] {
            assert_eq!(page(url), None, "{url}");
        }
    }
}
