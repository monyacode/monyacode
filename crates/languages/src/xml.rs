use anyhow::{Result, anyhow};
use async_trait::async_trait;
use gpui::AsyncApp;
use http_client::github::{AssetKind, GitHubLspBinaryVersion, latest_github_release};
use http_client::github_download::download_server_binary;
use language::{LanguageServerName, LspAdapter, LspAdapterDelegate, LspInstaller, Toolchain};
use lsp::LanguageServerBinary;
use smol::fs;
use std::path::PathBuf;
use util::fs::{make_file_executable, remove_matching};

use crate::helpers::{find_cached_server_binary, verify_metadata, with_exe, write_metadata};

pub struct XmlLspAdapter;

#[cfg(target_os = "macos")]
impl XmlLspAdapter {
    const OS_NAME: &str = "osx";
}

#[cfg(target_os = "linux")]
impl XmlLspAdapter {
    const OS_NAME: &str = "linux";
}

#[cfg(target_os = "windows")]
impl XmlLspAdapter {
    const OS_NAME: &str = "windows32";
}

impl XmlLspAdapter {
    const SERVER_NAME: LanguageServerName = LanguageServerName::new_static("lemminx");

    fn os_binary_stem(arch: &str) -> String {
        if Self::OS_NAME == "linux" || Self::OS_NAME == "osx" {
            format!("lemminx-{}-{}", Self::OS_NAME, arch)
        } else {
            format!("lemminx-{}", Self::OS_NAME)
        }
    }
}

impl LspInstaller for XmlLspAdapter {
    type BinaryVersion = GitHubLspBinaryVersion;

    async fn check_if_user_installed(
        &self,
        delegate: &dyn LspAdapterDelegate,
        _: Option<Toolchain>,
        _: &AsyncApp,
    ) -> Option<LanguageServerBinary> {
        let path = delegate.which("lemminx".as_ref()).await?;
        Some(LanguageServerBinary {
            path,
            arguments: Default::default(),
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
            latest_github_release("redhat-developer/vscode-xml", true, pre_release, delegate.http_client()).await?;

        let arch = match std::env::consts::ARCH {
            "aarch64" => "aarch_64",
            "x86_64" => "x86_64",
            other => return Err(anyhow!("unsupported architecture: {}", other)),
        };

        let os_name = Self::OS_NAME;
        if os_name != "osx" && arch == "aarch_64" {
            anyhow::bail!(
                "Lemminx does not provide prebuilt {arch}-binaries for {os_name} to fetch from GitHub. Consider installing the binary manually."
            )
        }

        let asset_name = PathBuf::from(Self::os_binary_stem(arch))
            .with_extension("zip")
            .to_str()
            // This cannot fail since os_binary_stem is guaranteed to return valid UTF-8
            .unwrap()
            .to_owned();

        let asset = release
            .assets
            .iter()
            .find(|a| a.name == asset_name)
            .ok_or_else(|| anyhow!("no matching asset found for {}", asset_name))?;

        Ok(GitHubLspBinaryVersion {
            name: release.tag_name,
            url: asset.browser_download_url.clone(),
            digest: asset.digest.clone(),
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

        let version_dir = container_dir.join(format!("lemminx-{version_name}"));
        let binary_path = version_dir.join(with_exe("lemminx"));

        let binary = LanguageServerBinary {
            path: binary_path.clone(),
            env: None,
            arguments: Default::default(),
        };

        if verify_metadata(&version_dir, &binary_path, &expected_digest, delegate).await {
            return Ok(binary);
        }

        download_server_binary(
            &*delegate.http_client(),
            &url,
            expected_digest.as_deref(),
            &version_dir,
            AssetKind::Zip,
        )
        .await?;

        // The extracted binary has an OS-specific name (e.g. "lemminx-linux",
        // "lemminx-osx-x86_64"). Rename it to the canonical "lemminx" name.
        let arch = std::env::consts::ARCH;
        let new_path = version_dir.join(with_exe(&Self::os_binary_stem(arch)));
        fs::rename(&new_path, &binary_path).await?;

        make_file_executable(&binary_path).await?;
        remove_matching(&container_dir, |path| path != version_dir).await;
        write_metadata(&version_dir, expected_digest).await?;

        Ok(binary)
    }

    async fn cached_server_binary(
        &self,
        container_dir: PathBuf,
        _: &dyn LspAdapterDelegate,
    ) -> Option<LanguageServerBinary> {
        match find_cached_server_binary(&container_dir, Some("lemminx-"), async |path| {
            Some(path.join(with_exe("lemminx")))
        })
        .await
        {
            Some(path) => Some(LanguageServerBinary {
                path,
                arguments: Default::default(),
                env: None,
            }),
            None => None,
        }
    }
}

#[async_trait(?Send)]
impl LspAdapter for XmlLspAdapter {
    fn name(&self) -> LanguageServerName {
        Self::SERVER_NAME
    }
}
