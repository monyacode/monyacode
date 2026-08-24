/// LSP support for Gleam
/// based on https://github.com/gleam-lang/zed
/// License: ./LICENSE-APACHE
/// Author: Marshall Bowers
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use gpui::AsyncApp;
use http_client::github::AssetKind;
use http_client::github::{GitHubLspBinaryVersion, latest_github_release};
use http_client::github_download::download_server_binary;
pub use language::*;
use lsp::{CompletionItemKind, LanguageServerBinary};
use project::ContextProviderWithTasks;
use std::path::PathBuf;
use std::sync::Arc;
use task::{TaskTemplate, TaskTemplates};
use util::fs::{make_file_executable, remove_matching};

use crate::helpers::{find_cached_server_binary, verify_metadata, with_exe, write_metadata};

pub struct GleamLspAdapter;

#[cfg(target_os = "macos")]
impl GleamLspAdapter {
    const GITHUB_ASSET_KIND: AssetKind = AssetKind::TarGz;
    const OS_NAME: &str = "apple-darwin";
}

#[cfg(target_os = "linux")]
impl GleamLspAdapter {
    const GITHUB_ASSET_KIND: AssetKind = AssetKind::TarGz;
    const OS_NAME: &str = "unknown-linux-musl";
}

#[cfg(target_os = "windows")]
impl GleamLspAdapter {
    const GITHUB_ASSET_KIND: AssetKind = AssetKind::Zip;
    const OS_NAME: &str = "pc-windows-msvc";
}

const SERVER_NAME: LanguageServerName = LanguageServerName::new_static("gleam");

impl LspInstaller for GleamLspAdapter {
    type BinaryVersion = GitHubLspBinaryVersion;

    async fn check_if_user_installed(
        &self,
        delegate: &dyn LspAdapterDelegate,
        _: Option<Toolchain>,
        _: &AsyncApp,
    ) -> Option<LanguageServerBinary> {
        let gleam_path = delegate.which(with_exe("gleam").as_ref()).await?;
        Some(LanguageServerBinary {
            path: gleam_path,
            arguments: vec!["lsp".into()],
            env: None,
        })
    }

    async fn fetch_latest_server_version(
        &self,
        delegate: &dyn LspAdapterDelegate,
        _pre_release: bool,
        _cx: &mut AsyncApp,
    ) -> Result<GitHubLspBinaryVersion> {
        let release = latest_github_release("gleam-lang/gleam", false, false, delegate.http_client()).await?;

        let arch = match std::env::consts::ARCH {
            "aarch64" => "aarch64",
            "x86" => "x86",
            "x86_64" => "x86_64",
            other => return Err(anyhow!("unsupported architecture: {}", other)),
        };

        let asset_name = format!(
            "gleam-{}-{}-{}.{}",
            release.tag_name,
            arch,
            Self::OS_NAME,
            match Self::GITHUB_ASSET_KIND {
                AssetKind::TarGz => "tar.gz",
                AssetKind::Zip => "zip",
                _ => unreachable!(),
            }
        );

        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| anyhow!("no matching asset found for {}", asset_name))?;

        Ok(GitHubLspBinaryVersion {
            name: release.tag_name.clone(),
            url: asset.browser_download_url.clone(),
            digest: None,
        })
    }

    async fn fetch_server_binary(
        &self,
        version: GitHubLspBinaryVersion,
        container_dir: PathBuf,
        delegate: &dyn LspAdapterDelegate,
    ) -> Result<LanguageServerBinary> {
        log::info!(
            "fetch_server_binary: version={:?} dir={:?}",
            version.name,
            container_dir
        );
        let GitHubLspBinaryVersion {
            name: version_name,
            url,
            digest: expected_digest,
        } = version;

        let destination_path = container_dir.join(format!("gleam-{version_name}"));
        let server_path = match Self::GITHUB_ASSET_KIND {
            AssetKind::TarGz => destination_path.join("gleam"),
            AssetKind::Zip => destination_path.join("gleam.exe"),
            _ => unreachable!(),
        };

        let binary = LanguageServerBinary {
            path: server_path.clone(),
            env: None,
            arguments: vec!["lsp".into()],
        };

        if verify_metadata(&destination_path, &server_path, &expected_digest, delegate).await {
            return Ok(binary);
        }
        download_server_binary(
            &*delegate.http_client(),
            &url,
            expected_digest.as_deref(),
            &destination_path,
            Self::GITHUB_ASSET_KIND,
        )
        .await?;
        make_file_executable(&server_path).await?;
        remove_matching(&container_dir, |path| path != destination_path).await;
        write_metadata(&destination_path, expected_digest).await?;
        Ok(binary)
    }

    async fn cached_server_binary(
        &self,
        container_dir: PathBuf,
        _: &dyn LspAdapterDelegate,
    ) -> Option<LanguageServerBinary> {
        match find_cached_server_binary(&container_dir, Some("gleam-"), async |path| {
            Some(path.join(with_exe("gleam")))
        })
        .await
        {
            Some(path) => Some(LanguageServerBinary {
                path,
                env: None,
                arguments: vec!["lsp".into()],
            }),
            None => None,
        }
    }
}

#[async_trait(?Send)]
impl LspAdapter for GleamLspAdapter {
    fn name(&self) -> LanguageServerName {
        SERVER_NAME
    }

    async fn label_for_completion(
        &self,
        completion: &lsp::CompletionItem,
        language: &Arc<Language>,
    ) -> Option<CodeLabel> {
        let label = &completion.label;
        let detail = completion.detail.as_ref()?;
        let ty = strip_newlines_from_detail(detail);

        let call_suffix = match completion.kind? {
            CompletionItemKind::FUNCTION | CompletionItemKind::CONSTRUCTOR => "()",
            _ => "",
        };

        let code = format!("{}{} {}", label, call_suffix, ty);

        let rope = Rope::from(code.as_str());
        let highlights = language.highlight_text(&rope, 0..code.len());

        let filter_range = completion
            .filter_text
            .as_deref()
            .and_then(|filter_text| code.find(filter_text).map(|start| start..start + filter_text.len()))
            .unwrap_or(0..label.len());

        Some(CodeLabel::new(code, filter_range, highlights))
    }
}

/// Removes newlines from the completion detail.
///
/// The Gleam LSP can return types containing newlines, which causes formatting
/// issues within the completions menu.
fn strip_newlines_from_detail(detail: &str) -> String {
    let without_newlines = detail.replace("->\n  ", "-> ").replace("\n  ", "").replace(",\n", "");

    without_newlines
        .split(',')
        .map(|part| part.trim())
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn gleam_task_context() -> ContextProviderWithTasks {
    ContextProviderWithTasks::new(TaskTemplates(vec![
        TaskTemplate {
            label: "gleam build".into(),
            command: "gleam".into(),
            args: vec!["build".into()],
            ..Default::default()
        },
        TaskTemplate {
            label: "gleam run".into(),
            command: "gleam".into(),
            args: vec!["run".into()],
            ..Default::default()
        },
        TaskTemplate {
            label: "gleam test".into(),
            command: "gleam".into(),
            args: vec!["test".into()],
            ..Default::default()
        },
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_newlines_from_detail() {
        let detail = "fn(\n  Selector(a),\n  b,\n  fn(Dynamic, Dynamic, Dynamic, Dynamic, Dynamic, Dynamic, Dynamic) -> a,\n) -> Selector(a)";
        let expected =
            "fn(Selector(a), b, fn(Dynamic, Dynamic, Dynamic, Dynamic, Dynamic, Dynamic, Dynamic) -> a) -> Selector(a)";
        assert_eq!(strip_newlines_from_detail(detail), expected);

        let detail =
            "fn(Selector(a), b, fn(Dynamic, Dynamic, Dynamic, Dynamic, Dynamic, Dynamic) -> a) ->\n  Selector(a)";
        let expected =
            "fn(Selector(a), b, fn(Dynamic, Dynamic, Dynamic, Dynamic, Dynamic, Dynamic) -> a) -> Selector(a)";
        assert_eq!(strip_newlines_from_detail(detail), expected);

        let detail = "fn(\n  Method,\n  List(#(String, String)),\n  a,\n  Scheme,\n  String,\n  Option(Int),\n  String,\n  Option(String),\n) -> Request(a)";
        let expected =
            "fn(Method, List(#(String, String)), a, Scheme, String, Option(Int), String, Option(String)) -> Request(a)";
        assert_eq!(strip_newlines_from_detail(detail), expected);
    }
}
