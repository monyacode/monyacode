use anyhow::{Result, anyhow};
use async_trait::async_trait;
use gpui::AsyncApp;
use http_client::github::{AssetKind, GitHubLspBinaryVersion, latest_github_release};
use http_client::github_download::download_server_binary;
use language::{LanguageServerName, LspAdapter, LspAdapterDelegate, LspInstaller, Toolchain};
use lsp::LanguageServerBinary;
use std::path::PathBuf;
use util::fs::make_file_executable;

use crate::helpers::{find_cached_server_binary, verify_metadata, write_metadata};

pub struct TomlLspAdapter;

#[cfg(target_os = "macos")]
impl TomlLspAdapter {
    const OS_NAME: &str = "darwin";
}

#[cfg(target_os = "linux")]
impl TomlLspAdapter {
    const OS_NAME: &str = "linux";
}

#[cfg(target_os = "windows")]
impl TomlLspAdapter {
    const OS_NAME: &str = "windows";
}

impl TomlLspAdapter {
    const SERVER_NAME: LanguageServerName = LanguageServerName::new_static("taplo");
}

impl LspInstaller for TomlLspAdapter {
    type BinaryVersion = GitHubLspBinaryVersion;

    async fn check_if_user_installed(
        &self,
        delegate: &dyn LspAdapterDelegate,
        _: Option<Toolchain>,
        _: &AsyncApp,
    ) -> Option<LanguageServerBinary> {
        let path = delegate.which("taplo".as_ref()).await?;
        Some(LanguageServerBinary {
            path,
            arguments: vec!["lsp".into(), "stdio".into()],
            env: None,
        })
    }

    async fn fetch_latest_server_version(
        &self,
        delegate: &dyn LspAdapterDelegate,
        pre_release: bool,
        _cx: &mut AsyncApp,
    ) -> Result<GitHubLspBinaryVersion> {
        let release = latest_github_release("tamasfe/taplo", true, pre_release, delegate.http_client()).await?;

        let arch = match std::env::consts::ARCH {
            "aarch64" => "aarch64",
            "x86_64" => "x86_64",
            "x86" => "x86",
            other => return Err(anyhow!("unsupported architecture: {}", other)),
        };

        let asset_name = format!("taplo-{}-{}.gz", Self::OS_NAME, arch);

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
        let GitHubLspBinaryVersion {
            name: version_name,
            url,
            digest: expected_digest,
        } = version;

        let path = container_dir.join(format!("taplo-{version_name}"));

        let binary = LanguageServerBinary {
            path: path.clone(),
            env: None,
            arguments: vec!["lsp".into(), "stdio".into()],
        };

        if verify_metadata(&path, &path, &expected_digest, delegate).await {
            return Ok(binary);
        }

        download_server_binary(
            &*delegate.http_client(),
            &url,
            expected_digest.as_deref(),
            &path,
            AssetKind::Gz,
        )
        .await?;

        make_file_executable(&path).await?;
        write_metadata(&path, expected_digest).await?;

        Ok(binary)
    }

    async fn cached_server_binary(
        &self,
        container_dir: PathBuf,
        _: &dyn LspAdapterDelegate,
    ) -> Option<LanguageServerBinary> {
        match find_cached_server_binary(&container_dir, Some("taplo-"), async |path| Some(path.clone())).await {
            Some(path) => Some(LanguageServerBinary {
                path,
                arguments: vec!["lsp".into(), "stdio".into()],
                env: None,
            }),
            None => None,
        }
    }
}

#[async_trait(?Send)]
impl LspAdapter for TomlLspAdapter {
    fn name(&self) -> LanguageServerName {
        Self::SERVER_NAME
    }
}
