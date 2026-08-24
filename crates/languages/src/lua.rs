/// LSP support for Lua
/// based on https://github.com/zed-extensions/lua
/// License: Apache-2.0
/// Author: Max Brunsfeld
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use gpui::AsyncApp;
use http_client::github::AssetKind;
use http_client::github::{GitHubLspBinaryVersion, latest_github_release};
use http_client::github_download::download_server_binary;
pub use language::*;
use lsp::{CompletionItemKind, LanguageServerBinary, SymbolKind};
use std::path::PathBuf;
use std::sync::Arc;
use util::fs::{make_file_executable, remove_matching};

use crate::helpers::{find_cached_server_binary, verify_metadata, with_exe, write_metadata};

pub struct LuaLspAdapter;

#[cfg(target_os = "macos")]
impl LuaLspAdapter {
    const OS_NAME: &str = "darwin";
    const ASSET_KIND: AssetKind = AssetKind::TarGz;
}

#[cfg(target_os = "linux")]
impl LuaLspAdapter {
    const OS_NAME: &str = "linux";
    const ASSET_KIND: AssetKind = AssetKind::TarGz;
}

#[cfg(target_os = "windows")]
impl LuaLspAdapter {
    const OS_NAME: &str = "win32";
    const ASSET_KIND: AssetKind = AssetKind::Zip;
}

const SERVER_NAME: LanguageServerName = LanguageServerName::new_static("lua-language-server");

impl LuaLspAdapter {
    fn binary_name() -> Option<String> {
        let arch = match std::env::consts::ARCH {
            "aarch64" => "arm64",
            "x86_64" => "x64",
            _ => return None,
        };
        Some(format!("lua-language-server-{}-{}", Self::OS_NAME, arch))
    }

    fn server_path(container: &PathBuf) -> Option<PathBuf> {
        Some(container.join("bin").join(with_exe("lua-language-server")))
    }
}

impl LspInstaller for LuaLspAdapter {
    type BinaryVersion = GitHubLspBinaryVersion;

    async fn check_if_user_installed(
        &self,
        delegate: &dyn LspAdapterDelegate,
        _: Option<Toolchain>,
        _: &AsyncApp,
    ) -> Option<LanguageServerBinary> {
        let path = delegate.which(with_exe("lua-language-server").as_ref()).await?;
        Some(LanguageServerBinary {
            path,
            arguments: vec![],
            env: None,
        })
    }

    async fn fetch_latest_server_version(
        &self,
        delegate: &dyn LspAdapterDelegate,
        pre_release: bool,
        _cx: &mut AsyncApp,
    ) -> Result<GitHubLspBinaryVersion> {
        let release =
            latest_github_release("LuaLS/lua-language-server", pre_release, true, delegate.http_client()).await?;

        let Some(binary_name) = Self::binary_name() else {
            return Err(anyhow!("unsupported architecture: {}", std::env::consts::ARCH));
        };

        let extension = match Self::ASSET_KIND {
            AssetKind::TarGz => "tar.gz",
            AssetKind::Zip => "zip",
            _ => unreachable!(),
        };

        let os = Self::OS_NAME;
        let arch = match std::env::consts::ARCH {
            "aarch64" => "arm64",
            "x86_64" => "x64",
            _ => return Err(anyhow!("unsupported architecture")),
        };
        let version = release.tag_name.clone();

        let asset_name = format!("lua-language-server-{version}-{os}-{arch}.{extension}");
        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| anyhow!("no matching asset found for {}", asset_name))?;

        Ok(GitHubLspBinaryVersion {
            name: binary_name,
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

        let destination_path = container_dir.join(format!("lua-language-server-{version_name}"));
        let server_path = Self::server_path(&destination_path).ok_or_else(|| anyhow!("Unsupported architecture"))?;

        let binary = LanguageServerBinary {
            path: server_path.clone(),
            arguments: vec![],
            env: None,
        };

        if verify_metadata(&destination_path, &server_path, &expected_digest, delegate).await {
            return Ok(binary);
        }

        download_server_binary(
            &*delegate.http_client(),
            &url,
            expected_digest.as_deref(),
            &destination_path,
            Self::ASSET_KIND,
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
        match find_cached_server_binary(&container_dir, Some("lua-language-server-"), async |path| {
            Self::server_path(path)
        })
        .await
        {
            Some(path) => Some(LanguageServerBinary {
                path,
                env: None,
                arguments: vec![],
            }),
            None => None,
        }
    }
}

#[async_trait(?Send)]
impl LspAdapter for LuaLspAdapter {
    fn name(&self) -> LanguageServerName {
        SERVER_NAME
    }

    async fn label_for_completion(
        &self,
        completion: &lsp::CompletionItem,
        language: &Arc<Language>,
    ) -> Option<CodeLabel> {
        let label = &completion.label;
        match completion.kind {
            Some(CompletionItemKind::METHOD | CompletionItemKind::FUNCTION) => {
                let name_len = label.find('(').unwrap_or(label.len());
                let source = Rope::from(label.as_str());
                let runs = language.highlight_text(&source, 0..label.len());
                let text = label.clone();
                let filter_range = 0..name_len;
                Some(CodeLabel::new(text, filter_range, runs))
            }
            Some(CompletionItemKind::FIELD) => {
                let source = Rope::from(label.as_str());
                let runs = language.highlight_text(&source, 0..label.len());
                let text = label.clone();
                let filter_range = 0..label.len();
                Some(CodeLabel::new(text, filter_range, runs))
            }
            _ => None,
        }
    }

    async fn label_for_symbol(&self, name: &str, kind: SymbolKind, language: &Arc<Language>) -> Option<CodeLabel> {
        let suffix = match kind {
            SymbolKind::METHOD => "()",
            _ => "",
        };
        return Some(CodeLabel::new(
            name.to_string(),
            0..name.len(),
            language.highlight_text(&Rope::from_iter([name, suffix]), 0..name.len()),
        ));
    }
}
