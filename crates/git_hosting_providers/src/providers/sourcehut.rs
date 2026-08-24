use std::str::FromStr;

use anyhow::{Result, bail};
use url::Url;

use git::{BuildCommitPermalinkParams, BuildPermalinkParams, GitHostingProvider, ParsedGitRemote, RemoteUrl};

use crate::get_host_from_git_remote_url;

#[derive(Debug)]
pub struct Sourcehut {
    name: String,
    base_url: Url,
}

impl Sourcehut {
    pub fn new(name: impl Into<String>, base_url: Url) -> Self {
        Self {
            name: name.into(),
            base_url,
        }
    }

    pub fn public_instance() -> Self {
        Self::new("Sourcehut", Url::parse("https://git.sr.ht").unwrap())
    }

    pub fn from_remote_url(remote_url: &str) -> Result<Self> {
        let host = get_host_from_git_remote_url(remote_url)?;
        if host == "git.sr.ht" {
            bail!("the Sourcehut instance is not self-hosted");
        }

        if !host.contains("git.sr.ht") {
            bail!("not a Sourcehut URL");
        }

        Ok(Self::new(
            "Sourcehut Self-Hosted",
            Url::parse(&format!("https://{}", host))?,
        ))
    }
}

impl GitHostingProvider for Sourcehut {
    fn name(&self) -> String {
        self.name.clone()
    }

    fn base_url(&self) -> Url {
        self.base_url.clone()
    }

    fn format_line_number(&self, line: u32) -> String {
        format!("L{line}")
    }

    fn format_line_numbers(&self, start_line: u32, end_line: u32) -> String {
        format!("L{start_line}-{end_line}")
    }

    fn parse_remote_url(&self, url: &str) -> Option<ParsedGitRemote> {
        let url = RemoteUrl::from_str(url).ok()?;

        let host = url.host_str()?;
        if host != self.base_url.host_str()? {
            return None;
        }

        let mut path_segments = url.path_segments()?;
        let owner = path_segments.next()?.trim_start_matches('~');
        // We don't trim the `.git` suffix here like we do elsewhere, as
        // sourcehut treats a repo with `.git` suffix as a separate repo.
        //
        // For example, `git@git.sr.ht:~username/repo` and `git@git.sr.ht:~username/repo.git`
        // are two distinct repositories.
        let repo = path_segments.next()?;

        Some(ParsedGitRemote {
            owner: owner.into(),
            repo: repo.into(),
        })
    }

    fn build_commit_permalink(&self, remote: &ParsedGitRemote, params: BuildCommitPermalinkParams) -> Url {
        let BuildCommitPermalinkParams { sha } = params;
        let ParsedGitRemote { owner, repo } = remote;

        self.base_url().join(&format!("~{owner}/{repo}/commit/{sha}")).unwrap()
    }

    fn build_permalink(&self, remote: ParsedGitRemote, params: BuildPermalinkParams) -> Url {
        let ParsedGitRemote { owner, repo } = remote;
        let BuildPermalinkParams { sha, path, selection } = params;

        let mut permalink = self
            .base_url()
            .join(&format!("~{owner}/{repo}/tree/{sha}/item/{path}"))
            .unwrap();
        permalink.set_fragment(selection.map(|selection| self.line_fragment(&selection)).as_deref());
        permalink
    }
}

#[cfg(test)]
mod tests {
    use git::repository::repo_path;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_parse_remote_url_given_ssh_url() {
        let parsed_remote = Sourcehut::public_instance()
            .parse_remote_url("git@git.sr.ht:~monyacode/monyacode")
            .unwrap();

        assert_eq!(
            parsed_remote,
            ParsedGitRemote {
                owner: "MonyaCode".into(),
                repo: "monyacode".into(),
            }
        );
    }

    #[test]
    fn test_parse_remote_url_given_ssh_url_with_git_suffix() {
        let parsed_remote = Sourcehut::public_instance()
            .parse_remote_url("git@git.sr.ht:~monyacode/monyacode.git")
            .unwrap();

        assert_eq!(
            parsed_remote,
            ParsedGitRemote {
                owner: "MonyaCode".into(),
                repo: "monyacode.git".into(),
            }
        );
    }

    #[test]
    fn test_parse_remote_url_given_https_url() {
        let parsed_remote = Sourcehut::public_instance()
            .parse_remote_url("https://git.sr.ht/~monyacode/monyacode")
            .unwrap();

        assert_eq!(
            parsed_remote,
            ParsedGitRemote {
                owner: "MonyaCode".into(),
                repo: "monyacode".into(),
            }
        );
    }

    #[test]
    fn test_build_sourcehut_permalink() {
        let permalink = Sourcehut::public_instance().build_permalink(
            ParsedGitRemote {
                owner: "MonyaCode".into(),
                repo: "monyacode".into(),
            },
            BuildPermalinkParams::new(
                "faa6f979be417239b2e070dbbf6392b909224e0b",
                &repo_path("crates/editor/src/git/permalink.rs"),
                None,
            ),
        );

        let expected_url = "https://git.sr.ht/~monyacode/monyacode/tree/faa6f979be417239b2e070dbbf6392b909224e0b/item/crates/editor/src/git/permalink.rs";
        assert_eq!(permalink.to_string(), expected_url.to_string())
    }

    #[test]
    fn test_build_sourcehut_permalink_with_git_suffix() {
        let permalink = Sourcehut::public_instance().build_permalink(
            ParsedGitRemote {
                owner: "MonyaCode".into(),
                repo: "monyacode.git".into(),
            },
            BuildPermalinkParams::new(
                "faa6f979be417239b2e070dbbf6392b909224e0b",
                &repo_path("crates/editor/src/git/permalink.rs"),
                None,
            ),
        );

        let expected_url = "https://git.sr.ht/~monyacode/monyacode.git/tree/faa6f979be417239b2e070dbbf6392b909224e0b/item/crates/editor/src/git/permalink.rs";
        assert_eq!(permalink.to_string(), expected_url.to_string())
    }

    #[test]
    fn test_build_sourcehut_permalink_with_single_line_selection() {
        let permalink = Sourcehut::public_instance().build_permalink(
            ParsedGitRemote {
                owner: "MonyaCode".into(),
                repo: "monyacode".into(),
            },
            BuildPermalinkParams::new(
                "faa6f979be417239b2e070dbbf6392b909224e0b",
                &repo_path("crates/editor/src/git/permalink.rs"),
                Some(6..6),
            ),
        );

        let expected_url = "https://git.sr.ht/~monyacode/monyacode/tree/faa6f979be417239b2e070dbbf6392b909224e0b/item/crates/editor/src/git/permalink.rs#L7";
        assert_eq!(permalink.to_string(), expected_url.to_string())
    }

    #[test]
    fn test_build_sourcehut_permalink_with_multi_line_selection() {
        let permalink = Sourcehut::public_instance().build_permalink(
            ParsedGitRemote {
                owner: "MonyaCode".into(),
                repo: "monyacode".into(),
            },
            BuildPermalinkParams::new(
                "faa6f979be417239b2e070dbbf6392b909224e0b",
                &repo_path("crates/editor/src/git/permalink.rs"),
                Some(23..47),
            ),
        );

        let expected_url = "https://git.sr.ht/~monyacode/monyacode/tree/faa6f979be417239b2e070dbbf6392b909224e0b/item/crates/editor/src/git/permalink.rs#L24-48";
        assert_eq!(permalink.to_string(), expected_url.to_string())
    }
}
