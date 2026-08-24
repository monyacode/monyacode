use std::str::FromStr;
use std::sync::LazyLock;

use anyhow::{Result, bail};
use async_trait::async_trait;
use regex::Regex;
use url::Url;
use urlencoding::encode;

use git::{
    BuildCommitPermalinkParams, BuildPermalinkParams, GitHostingProvider, ParsedGitRemote, PullRequest, RemoteUrl,
};

use crate::get_host_from_git_remote_url;

fn pull_request_number_regex() -> &'static Regex {
    static PULL_REQUEST_NUMBER_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\(#(\d+)\)$").unwrap());
    &PULL_REQUEST_NUMBER_REGEX
}

#[derive(Debug)]
pub struct Github {
    name: String,
    base_url: Url,
}

impl Github {
    pub fn new(name: impl Into<String>, base_url: Url) -> Self {
        Self {
            name: name.into(),
            base_url,
        }
    }

    pub fn public_instance() -> Self {
        Self::new("GitHub", Url::parse("https://github.com").unwrap())
    }

    pub fn from_remote_url(remote_url: &str) -> Result<Self> {
        let host = get_host_from_git_remote_url(remote_url)?;
        if host == "github.com" {
            bail!("the GitHub instance is not self-hosted");
        }

        // TODO: detecting self hosted instances by checking whether "github" is in the url or not
        // is not very reliable. See https://github.com/zed-industries/zed/issues/26393 for more
        // information.
        if !host.contains("github") {
            bail!("not a GitHub URL");
        }

        Ok(Self::new(
            "GitHub Self-Hosted",
            Url::parse(&format!("https://{}", host))?,
        ))
    }
}

#[async_trait]
impl GitHostingProvider for Github {
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
        let mut owner = path_segments.next()?;
        if owner.is_empty() {
            owner = path_segments.next()?;
        }

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
            .join(&format!("{owner}/{repo}/blob/{sha}/{path}"))
            .unwrap();
        if path.ends_with(".md") {
            permalink.set_query(Some("plain=1"));
        }
        permalink.set_fragment(selection.map(|selection| self.line_fragment(&selection)).as_deref());
        permalink
    }

    fn build_create_pull_request_url(&self, remote: &ParsedGitRemote, source_branch: &str) -> Option<Url> {
        let ParsedGitRemote { owner, repo } = remote;
        let encoded_source = encode(source_branch);

        self.base_url()
            .join(&format!("{owner}/{repo}/pull/new/{encoded_source}"))
            .ok()
    }

    fn extract_pull_request(&self, remote: &ParsedGitRemote, message: &str) -> Option<PullRequest> {
        let line = message.lines().next()?;
        let capture = pull_request_number_regex().captures(line)?;
        let number = capture.get(1)?.as_str().parse::<u32>().ok()?;

        let mut url = self.base_url();
        let path = format!("/{}/{}/pull/{}", remote.owner, remote.repo, number);
        url.set_path(&path);

        Some(PullRequest { number, url })
    }
}

#[cfg(test)]
mod tests {
    use git::repository::repo_path;
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    use super::*;

    #[test]
    fn test_remote_url_with_root_slash() {
        let remote_url = "git@github.com:/monyacode/monyacode";
        let parsed_remote = Github::public_instance().parse_remote_url(remote_url).unwrap();

        assert_eq!(
            parsed_remote,
            ParsedGitRemote {
                owner: "MonyaCode".into(),
                repo: "monyacode".into(),
            }
        );
    }

    #[test]
    fn test_invalid_self_hosted_remote_url() {
        let remote_url = "git@github.com:monyacode/monyacode.git";
        let github = Github::from_remote_url(remote_url);
        assert!(github.is_err());
    }

    #[test]
    fn test_from_remote_url_ssh() {
        let remote_url = "git@github.my-enterprise.com:monyacode/monyacode.git";
        let github = Github::from_remote_url(remote_url).unwrap();

        assert_eq!(github.name, "GitHub Self-Hosted".to_string());
        assert_eq!(github.base_url, Url::parse("https://github.my-enterprise.com").unwrap());
    }

    #[test]
    fn test_from_remote_url_https() {
        let remote_url = "https://github.my-enterprise.com/monyacode/monyacode.git";
        let github = Github::from_remote_url(remote_url).unwrap();

        assert_eq!(github.name, "GitHub Self-Hosted".to_string());
        assert_eq!(github.base_url, Url::parse("https://github.my-enterprise.com").unwrap());
    }

    #[test]
    fn test_parse_remote_url_given_self_hosted_ssh_url() {
        let remote_url = "git@github.my-enterprise.com:monyacode/monyacode.git";
        let parsed_remote = Github::from_remote_url(remote_url)
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
    fn test_parse_remote_url_given_self_hosted_https_url_with_subgroup() {
        let remote_url = "https://github.my-enterprise.com/monyacode/monyacode.git";
        let parsed_remote = Github::from_remote_url(remote_url)
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
    fn test_parse_remote_url_given_ssh_url() {
        let parsed_remote = Github::public_instance()
            .parse_remote_url("git@github.com:monyacode/monyacode.git")
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
        let parsed_remote = Github::public_instance()
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
    fn test_parse_remote_url_given_https_url_with_username() {
        let parsed_remote = Github::public_instance()
            .parse_remote_url("https://jlannister@github.com/some-org/some-repo.git")
            .unwrap();

        assert_eq!(
            parsed_remote,
            ParsedGitRemote {
                owner: "some-org".into(),
                repo: "some-repo".into(),
            }
        );
    }

    #[test]
    fn test_build_github_permalink_from_ssh_url() {
        let remote = ParsedGitRemote {
            owner: "MonyaCode".into(),
            repo: "monyacode".into(),
        };
        let permalink = Github::public_instance().build_permalink(
            remote,
            BuildPermalinkParams::new(
                "e6ebe7974deb6bb6cc0e2595c8ec31f0c71084b7",
                &repo_path("crates/editor/src/git/permalink.rs"),
                None,
            ),
        );

        let expected_url = "https://github.com/monyacode/monyacode/blob/e6ebe7974deb6bb6cc0e2595c8ec31f0c71084b7/crates/editor/src/git/permalink.rs";
        assert_eq!(permalink.to_string(), expected_url.to_string())
    }

    #[test]
    fn test_build_github_permalink() {
        let permalink = Github::public_instance().build_permalink(
            ParsedGitRemote {
                owner: "MonyaCode".into(),
                repo: "monyacode".into(),
            },
            BuildPermalinkParams::new(
                "b2efec9824c45fcc90c9a7eb107a50d1772a60aa",
                &repo_path("crates/zed/src/main.rs"),
                None,
            ),
        );

        let expected_url = "https://github.com/monyacode/monyacode/blob/b2efec9824c45fcc90c9a7eb107a50d1772a60aa/crates/zed/src/main.rs";
        assert_eq!(permalink.to_string(), expected_url.to_string())
    }

    #[test]
    fn test_build_github_permalink_with_single_line_selection() {
        let permalink = Github::public_instance().build_permalink(
            ParsedGitRemote {
                owner: "MonyaCode".into(),
                repo: "monyacode".into(),
            },
            BuildPermalinkParams::new(
                "e6ebe7974deb6bb6cc0e2595c8ec31f0c71084b7",
                &repo_path("crates/editor/src/git/permalink.rs"),
                Some(6..6),
            ),
        );

        let expected_url = "https://github.com/monyacode/monyacode/blob/e6ebe7974deb6bb6cc0e2595c8ec31f0c71084b7/crates/editor/src/git/permalink.rs#L7";
        assert_eq!(permalink.to_string(), expected_url.to_string())
    }

    #[test]
    fn test_build_github_permalink_with_multi_line_selection() {
        let permalink = Github::public_instance().build_permalink(
            ParsedGitRemote {
                owner: "MonyaCode".into(),
                repo: "monyacode".into(),
            },
            BuildPermalinkParams::new(
                "e6ebe7974deb6bb6cc0e2595c8ec31f0c71084b7",
                &repo_path("crates/editor/src/git/permalink.rs"),
                Some(23..47),
            ),
        );

        let expected_url = "https://github.com/monyacode/monyacode/blob/e6ebe7974deb6bb6cc0e2595c8ec31f0c71084b7/crates/editor/src/git/permalink.rs#L24-L48";
        assert_eq!(permalink.to_string(), expected_url.to_string())
    }

    #[test]
    fn test_build_github_create_pr_url() {
        let remote = ParsedGitRemote {
            owner: "MonyaCode".into(),
            repo: "monyacode".into(),
        };

        let provider = Github::public_instance();

        let url = provider
            .build_create_pull_request_url(&remote, "feature/something cool")
            .expect("url should be constructed");

        assert_eq!(
            url.as_str(),
            "https://github.com/monyacode/monyacode/pull/new/feature%2Fsomething%20cool"
        );
    }

    #[test]
    fn test_github_pull_requests() {
        let remote = ParsedGitRemote {
            owner: "MonyaCode".into(),
            repo: "monyacode".into(),
        };

        let github = Github::public_instance();
        let message = "This does not contain a pull request";
        assert!(github.extract_pull_request(&remote, message).is_none());

        // Pull request number at end of first line
        let message = indoc! {r#"
            project panel: do not expand collapsed worktrees on "collapse all entries" (#10687)

            Fixes #10597

            Release Notes:

            - Fixed "project panel: collapse all entries" expanding collapsed worktrees.
            "#
        };

        assert_eq!(
            github.extract_pull_request(&remote, message).unwrap().url.as_str(),
            "https://github.com/monyacode/monyacode/pull/10687"
        );

        // Pull request number in middle of line, which we want to ignore
        let message = indoc! {r#"
            Follow-up to #10687 to fix problems

            See the original PR, this is a fix.
            "#
        };
        assert_eq!(github.extract_pull_request(&remote, message), None);
    }

    /// Regression test for issue #39875
    #[test]
    fn test_git_permalink_url_escaping() {
        let permalink = Github::public_instance().build_permalink(
            ParsedGitRemote {
                owner: "MonyaCode".into(),
                repo: "nonexistent".into(),
            },
            BuildPermalinkParams::new(
                "3ef1539900037dd3601be7149b2b39ed6d0ce3db",
                &repo_path("app/blog/[slug]/page.tsx"),
                Some(7..7),
            ),
        );

        let expected_url = "https://github.com/MonyaCode/nonexistent/blob/3ef1539900037dd3601be7149b2b39ed6d0ce3db/app/blog/%5Bslug%5D/page.tsx#L8";
        assert_eq!(permalink.to_string(), expected_url.to_string())
    }

    #[test]
    fn test_build_create_pull_request_url() {
        let remote = ParsedGitRemote {
            owner: "MonyaCode".into(),
            repo: "monyacode".into(),
        };

        let github = Github::public_instance();
        let url = github
            .build_create_pull_request_url(&remote, "feature/new-feature")
            .unwrap();

        assert_eq!(
            url.as_str(),
            "https://github.com/monyacode/monyacode/pull/new/feature%2Fnew-feature"
        );

        let base_url = Url::parse("https://github.eat-the-rich.com").unwrap();
        let github = Github::new("GitHub Self-Hosted", base_url);
        let url = github
            .build_create_pull_request_url(&remote, "feature/new-feature")
            .expect("should be able to build pull request url");

        assert_eq!(
            url.as_str(),
            "https://github.eat-the-rich.com/monyacode/monyacode/pull/new/feature%2Fnew-feature"
        );
    }
}
