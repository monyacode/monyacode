use std::str::FromStr;

use anyhow::{Result, bail};
use async_trait::async_trait;
use url::Url;

use git::{BuildCommitPermalinkParams, BuildPermalinkParams, GitHostingProvider, ParsedGitRemote, RemoteUrl};

use crate::get_host_from_git_remote_url;

pub struct Forgejo {
    name: String,
    base_url: Url,
}

impl Forgejo {
    pub fn new(name: impl Into<String>, base_url: Url) -> Self {
        Self {
            name: name.into(),
            base_url,
        }
    }

    pub fn public_instance() -> Self {
        Self::new("Codeberg", Url::parse("https://codeberg.org").unwrap())
    }

    pub fn from_remote_url(remote_url: &str) -> Result<Self> {
        let host = get_host_from_git_remote_url(remote_url)?;
        if host == "codeberg.org" {
            bail!("the Forgejo instance is not self-hosted");
        }

        // TODO: detecting self hosted instances by checking whether "forgejo" is in the url or not
        // is not very reliable. See https://github.com/zed-industries/zed/issues/26393 for more
        // information.
        if !host.contains("forgejo") && !host.contains("git.") {
            bail!("not a Forgejo URL");
        }

        Ok(Self::new(
            "Forgejo Self-Hosted",
            Url::parse(&format!("https://{}", host))?,
        ))
    }
}

#[async_trait]
impl GitHostingProvider for Forgejo {
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
        format!("L{start_line}-L{end_line}")
    }

    fn parse_remote_url(&self, url: &str) -> Option<ParsedGitRemote> {
        let url = RemoteUrl::from_str(url).ok()?;

        let host = url.host_str()?;
        if host != self.base_url.host_str()? {
            return None;
        }

        let mut path_segments = url.path_segments()?;
        let owner = path_segments.next()?;
        let repo = path_segments.next()?.trim_end_matches(".git");

        Some(ParsedGitRemote {
            owner: owner.into(),
            repo: repo.into(),
        })
    }

    fn build_commit_permalink(&self, remote: &ParsedGitRemote, params: BuildCommitPermalinkParams) -> Url {
        let BuildCommitPermalinkParams { sha } = params;
        let ParsedGitRemote { owner, repo } = remote;

        self.base_url().join(&format!("{owner}/{repo}/commit/{sha}")).unwrap()
    }

    fn build_permalink(&self, remote: ParsedGitRemote, params: BuildPermalinkParams) -> Url {
        let ParsedGitRemote { owner, repo } = remote;
        let BuildPermalinkParams { sha, path, selection } = params;

        let mut permalink = self
            .base_url()
            .join(&format!("{owner}/{repo}/src/commit/{sha}/{path}"))
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
        let parsed_remote = Forgejo::public_instance()
            .parse_remote_url("git@codeberg.org:monyacode/monyacode.git")
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
    fn test_parse_remote_url_given_https_url() {
        let parsed_remote = Forgejo::public_instance()
            .parse_remote_url("https://github.com/monyacode/monyacode.git")
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
    fn test_parse_remote_url_given_self_hosted_ssh_url() {
        let remote_url = "git@forgejo.my-enterprise.com:monyacode/monyacode.git";

        let parsed_remote = Forgejo::from_remote_url(remote_url)
            .unwrap()
            .parse_remote_url(remote_url)
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
    fn test_parse_remote_url_given_self_hosted_https_url() {
        let remote_url = "https://forgejo.my-enterprise.com/monyacode/monyacode.git";
        let parsed_remote = Forgejo::from_remote_url(remote_url)
            .unwrap()
            .parse_remote_url(remote_url)
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
    fn test_build_codeberg_permalink() {
        let permalink = Forgejo::public_instance().build_permalink(
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

        let expected_url = "https://github.com/monyacode/monyacode/src/commit/faa6f979be417239b2e070dbbf6392b909224e0b/crates/editor/src/git/permalink.rs";
        assert_eq!(permalink.to_string(), expected_url.to_string())
    }

    #[test]
    fn test_build_codeberg_permalink_with_single_line_selection() {
        let permalink = Forgejo::public_instance().build_permalink(
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

        let expected_url = "https://github.com/monyacode/monyacode/src/commit/faa6f979be417239b2e070dbbf6392b909224e0b/crates/editor/src/git/permalink.rs#L7";
        assert_eq!(permalink.to_string(), expected_url.to_string())
    }

    #[test]
    fn test_build_codeberg_permalink_with_multi_line_selection() {
        let permalink = Forgejo::public_instance().build_permalink(
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

        let expected_url = "https://github.com/monyacode/monyacode/src/commit/faa6f979be417239b2e070dbbf6392b909224e0b/crates/editor/src/git/permalink.rs#L24-L48";
        assert_eq!(permalink.to_string(), expected_url.to_string())
    }

    #[test]
    fn test_build_forgejo_self_hosted_permalink_from_ssh_url() {
        let forgejo = Forgejo::from_remote_url("git@forgejo.some-enterprise.com:monyacode/monyacode.git").unwrap();
        let permalink = forgejo.build_permalink(
            ParsedGitRemote {
                owner: "MonyaCode".into(),
                repo: "monyacode".into(),
            },
            BuildPermalinkParams::new(
                "e6ebe7974deb6bb6cc0e2595c8ec31f0c71084b7",
                &repo_path("crates/editor/src/git/permalink.rs"),
                None,
            ),
        );

        let expected_url = "https://forgejo.some-enterprise.com/monyacode/monyacode/src/commit/e6ebe7974deb6bb6cc0e2595c8ec31f0c71084b7/crates/editor/src/git/permalink.rs";
        assert_eq!(permalink.to_string(), expected_url.to_string())
    }

    #[test]
    fn test_build_forgejo_self_hosted_permalink_from_https_url() {
        let forgejo = Forgejo::from_remote_url("https://forgejo-instance.big-co.com/monyacode/monyacode.git").unwrap();
        let permalink = forgejo.build_permalink(
            ParsedGitRemote {
                owner: "MonyaCode".into(),
                repo: "monyacode".into(),
            },
            BuildPermalinkParams::new(
                "b2efec9824c45fcc90c9a7eb107a50d1772a60aa",
                &repo_path("crates/monyacode/src/main.rs"),
                None,
            ),
        );

        let expected_url = "https://forgejo-instance.big-co.com/monyacode/monyacode/src/commit/b2efec9824c45fcc90c9a7eb107a50d1772a60aa/crates/monyacode/src/main.rs";
        assert_eq!(permalink.to_string(), expected_url.to_string())
    }
}
