/// LSP support for Erlang
/// based on https://github.com/zed-extensions/erlang
/// License: Apache-2.0
/// Authors: Dairon M, Fabian Bergström
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use gpui::AsyncApp;
use http_client::github::AssetKind;
use http_client::github::{GitHubLspBinaryVersion, latest_github_release};
use http_client::github_download::download_server_binary;
pub use language::*;
use lsp::LanguageServerBinary;
use std::path::PathBuf;
use util::fs::{make_file_executable, remove_matching};

use crate::helpers::{find_cached_server_binary, verify_metadata, with_exe, write_metadata};

pub struct ErlangLsAdapter;

impl ErlangLsAdapter {
    const SERVER_NAME: LanguageServerName = LanguageServerName::new_static("erlang-ls");
}

impl LspInstaller for ErlangLsAdapter {
    type BinaryVersion = GitHubLspBinaryVersion;

    async fn check_if_user_installed(
        &self,
        delegate: &dyn LspAdapterDelegate,
        _: Option<Toolchain>,
        _: &AsyncApp,
    ) -> Option<LanguageServerBinary> {
        let path = delegate.which("erlang_ls".as_ref()).await?;
        Some(LanguageServerBinary {
            path,
            arguments: vec![],
            env: None,
        })
    }

    async fn fetch_latest_server_version(
        &self,
        _delegate: &dyn LspAdapterDelegate,
        _pre_release: bool,
        _cx: &mut AsyncApp,
    ) -> Result<GitHubLspBinaryVersion> {
        Err(anyhow!(
            "erlang-ls must be installed manually and available on your $PATH"
        ))
    }

    async fn fetch_server_binary(
        &self,
        _version: GitHubLspBinaryVersion,
        _container_dir: PathBuf,
        _delegate: &dyn LspAdapterDelegate,
    ) -> Result<LanguageServerBinary> {
        Err(anyhow!(
            "erlang-ls must be installed manually and available on your $PATH"
        ))
    }

    async fn cached_server_binary(
        &self,
        _container_dir: PathBuf,
        _delegate: &dyn LspAdapterDelegate,
    ) -> Option<LanguageServerBinary> {
        None
    }
}

#[async_trait(?Send)]
impl LspAdapter for ErlangLsAdapter {
    fn name(&self) -> LanguageServerName {
        Self::SERVER_NAME
    }
}

pub struct ElpAdapter;

#[cfg(target_os = "macos")]
impl ElpAdapter {
    const OS_NAME: &str = "macos";
    const OS_TARGET: &str = "apple-darwin";
}

#[cfg(target_os = "linux")]
impl ElpAdapter {
    const OS_NAME: &str = "linux";
    const OS_TARGET: &str = "unknown-linux-gnu";
}

impl ElpAdapter {
    const SERVER_NAME: LanguageServerName = LanguageServerName::new_static("elp");
    const OTP_VERSION: &str = "28";

    fn server_path(container: &PathBuf) -> Option<PathBuf> {
        Some(container.join("elp"))
    }
}

impl LspInstaller for ElpAdapter {
    type BinaryVersion = GitHubLspBinaryVersion;

    async fn check_if_user_installed(
        &self,
        delegate: &dyn LspAdapterDelegate,
        _: Option<Toolchain>,
        _: &AsyncApp,
    ) -> Option<LanguageServerBinary> {
        log::info!("checking pre-installed ELP...");
        let path = delegate.which(with_exe("elp").as_ref()).await?;
        Some(LanguageServerBinary {
            path,
            arguments: vec!["server".into()],
            env: None,
        })
    }

    #[cfg(target_os = "windows")]
    async fn fetch_latest_server_version(
        &self,
        delegate: &dyn LspAdapterDelegate,
        pre_release: bool,
        _cx: &mut AsyncApp,
    ) -> Result<GitHubLspBinaryVersion> {
        return Err(anyhow!("ELP is not supported on Windows"));
    }

    #[cfg(not(target_os = "windows"))]
    async fn fetch_latest_server_version(
        &self,
        delegate: &dyn LspAdapterDelegate,
        pre_release: bool,
        _cx: &mut AsyncApp,
    ) -> Result<GitHubLspBinaryVersion> {
        log::info!("trying github download for ELP...");
        let release = latest_github_release(
            "WhatsApp/erlang-language-platform",
            pre_release,
            true,
            delegate.http_client(),
        )
        .await?;

        let arch = match std::env::consts::ARCH {
            "aarch64" => "aarch64",
            "x86_64" => "x86_64",
            other => return Err(anyhow!("unsupported architecture: {}", other)),
        };

        let asset_name = format!(
            "elp-{}-{}-{}-otp-{}.tar.gz",
            Self::OS_NAME,
            arch,
            Self::OS_TARGET,
            Self::OTP_VERSION
        );

        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| anyhow!("no matching asset found for {}", asset_name))?;

        log::info!("Found {asset:?}");

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

        let destination_path = container_dir.join(format!("elp-{version_name}"));
        let server_path =
            Self::server_path(&destination_path).ok_or_else(|| anyhow!("Could not determine server path"))?;

        let binary = LanguageServerBinary {
            path: server_path.clone(),
            arguments: vec!["server".into()],
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
        match find_cached_server_binary(&container_dir, Some("elp-"), async |path| Self::server_path(path)).await {
            Some(path) => Some(LanguageServerBinary {
                path,
                arguments: vec!["server".into()],
                env: None,
            }),
            None => None,
        }
    }
}

#[async_trait(?Send)]
impl LspAdapter for ElpAdapter {
    fn name(&self) -> LanguageServerName {
        Self::SERVER_NAME
    }
}
