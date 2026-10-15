//! Repo identity (SPEC §6.1): remote URL → git common dir → cwd, hashed to a key.

use std::path::Path;
use std::process::Command;

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Repo {
    /// First 16 hex chars of SHA-256 of the identity string.
    pub key: String,
    /// Short display name (last path segment).
    pub name: String,
}

impl Repo {
    pub fn from_identity(identity: &str) -> Self {
        let digest = Sha256::digest(identity.as_bytes());
        let key = digest[..8].iter().map(|b| format!("{b:02x}")).collect();
        let name = identity
            .trim_end_matches(['/', '\\'])
            .rsplit(['/', '\\'])
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or(identity)
            .to_string();
        Repo { key, name }
    }
}

/// `https://User@GitHub.com/Owner/repo.git`, `git@github.com:Owner/repo.git`, and
/// `ssh://git@github.com/Owner/repo` all become `github.com/Owner/repo`.
pub fn normalize_remote(url: &str) -> String {
    let mut s = url.trim();
    if let Some((_, rest)) = s.split_once("://") {
        s = rest;
    }
    if let Some((user, rest)) = s.split_once('@')
        && !user.contains('/')
    {
        s = rest;
    }
    let s = s.strip_suffix(".git").unwrap_or(s).trim_end_matches('/');
    // scp-like `host:path` (but not a Windows drive like `C:\...`)
    let (host, path) = match s.split_once(['/', ':']) {
        Some((h, p)) if h.len() > 1 => (h, p),
        _ => return s.to_string(),
    };
    format!("{}/{}", host.to_lowercase(), path.trim_start_matches('/'))
}

fn git(cwd: &Path, args: &[&str]) -> Option<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .ok()?;
    let s = String::from_utf8(out.stdout).ok()?.trim().to_string();
    (out.status.success() && !s.is_empty()).then_some(s)
}

/// Resolve the repo for an agent's cwd. Runs `git`, so call it off the render thread
/// and cache per cwd.
pub fn identify(cwd: &Path) -> Repo {
    if let Some(url) = git(cwd, &["config", "--get", "remote.origin.url"]) {
        return Repo::from_identity(&normalize_remote(&url));
    }
    let common = git(
        cwd,
        &["rev-parse", "--path-format=absolute", "--git-common-dir"],
    );
    if let Some(dir) = common {
        // `/x/myrepo/.git` → identity is the dir, name comes from its parent.
        let dir = dir.replace('\\', "/");
        let mut r = Repo::from_identity(&dir);
        if let Some(parent) = dir.strip_suffix("/.git") {
            r.name = Repo::from_identity(parent).name;
        }
        return r;
    }
    Repo::from_identity(&cwd.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remotes_normalize_to_one_identity() {
        let want = "github.com/KantaKan/herdr-fantasy";
        for url in [
            "https://github.com/KantaKan/herdr-fantasy.git",
            "https://User@GitHub.com/KantaKan/herdr-fantasy",
            "git@github.com:KantaKan/herdr-fantasy.git",
            "ssh://git@github.com/KantaKan/herdr-fantasy/",
        ] {
            assert_eq!(normalize_remote(url), want, "{url}");
        }
    }

    #[test]
    fn key_and_name() {
        let r = Repo::from_identity("github.com/KantaKan/herdr-fantasy");
        assert_eq!(r.key.len(), 16);
        assert_eq!(r.name, "herdr-fantasy");
        assert_eq!(Repo::from_identity(r"D:\code\baro").name, "baro");
        assert_ne!(r.key, Repo::from_identity("github.com/x/y").key);
    }

    #[test]
    fn this_checkout_is_identified_by_its_remote() {
        let r = identify(Path::new(env!("CARGO_MANIFEST_DIR")));
        assert!(!r.name.is_empty());
    }
}
