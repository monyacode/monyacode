use anyhow::{Result, anyhow};
use async_trait::async_trait;
use gpui::AsyncApp;
use http_client::github::{AssetKind, GitHubLspBinaryVersion, latest_github_release};
use http_client::github_download::download_server_binary;
use language::{LanguageServerName, LspAdapter, LspAdapterDelegate, LspInstaller, Toolchain};
use lsp::LanguageServerBinary;
use std::path::PathBuf;
use util::fs::{make_file_executable, remove_matching};

use crate::helpers::{find_cached_server_binary, verify_metadata, with_exe, write_metadata};

pub struct OpenTofuLspAdapter;

#[cfg(target_os = "macos")]
impl OpenTofuLspAdapter {
    const OS_NAME: &str = "Darwin";
}

#[cfg(target_os = "linux")]
impl OpenTofuLspAdapter {
    const OS_NAME: &str = "Linux";
}

#[cfg(target_os = "windows")]
impl OpenTofuLspAdapter {
    const OS_NAME: &str = "Windows";
}

impl OpenTofuLspAdapter {
    const SERVER_NAME: LanguageServerName = LanguageServerName::new_static("tofu-ls");
}

impl LspInstaller for OpenTofuLspAdapter {
    type BinaryVersion = GitHubLspBinaryVersion;

    async fn check_if_user_installed(
        &self,
        delegate: &dyn LspAdapterDelegate,
        _: Option<Toolchain>,
        _: &AsyncApp,
    ) -> Option<LanguageServerBinary> {
        let path = delegate.which("tofu-ls".as_ref()).await?;
        Some(LanguageServerBinary {
            path,
            arguments: vec!["serve".into()],
            env: None,
        })
    }

    async fn fetch_latest_server_version(
        &self,
        delegate: &dyn LspAdapterDelegate,
        pre_release: bool,
        _cx: &mut AsyncApp,
    ) -> Result<GitHubLspBinaryVersion> {
        let release = latest_github_release("opentofu/tofu-ls", true, pre_release, delegate.http_client()).await?;

        let arch = match std::env::consts::ARCH {
            "aarch64" => "arm64",
            "x86_64" => "x86_64",
            "x86" => "i386",
            other => return Err(anyhow!("unsupported architecture: {}", other)),
        };

        let asset_name = format!("tofu-ls_{}_{}.tar.gz", Self::OS_NAME, arch);

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

        let destination_path = container_dir.join(format!("tofu-ls-{version_name}"));
        let server_path = destination_path.join(with_exe("tofu-ls"));

        let binary = LanguageServerBinary {
            path: server_path.clone(),
            env: None,
            arguments: vec!["serve".into()],
        };

        if verify_metadata(&destination_path, &server_path, &expected_digest, delegate).await {
            return Ok(binary);
        }

        download_server_binary(
            &*delegate.http_client(),
            &url,
            expected_digest.as_deref(),
            &destination_path,
            AssetKind::TarGz,
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
        match find_cached_server_binary(&container_dir, Some("tofu-ls-"), async |path| {
            Some(path.join(with_exe("tofu-ls")))
        })
        .await
        {
            Some(path) => Some(LanguageServerBinary {
                path,
                arguments: vec!["serve".into()],
                env: None,
            }),
            None => None,
        }
    }
}

#[async_trait(?Send)]
impl LspAdapter for OpenTofuLspAdapter {
    fn name(&self) -> LanguageServerName {
        Self::SERVER_NAME
    }
}
