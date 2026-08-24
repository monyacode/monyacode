use crate::helpers::{find_cached_server_binary, with_exe};
#[cfg(not(target_os = "linux"))]
use crate::helpers::{verify_metadata, write_metadata};
use anyhow::Result;
#[cfg(not(target_os = "linux"))]
use anyhow::anyhow;
use async_trait::async_trait;
use gpui::AsyncApp;
#[cfg(target_os = "linux")]
use http_client::github::GitHubLspBinaryVersion;
#[cfg(not(target_os = "linux"))]
use http_client::github::{AssetKind, GitHubLspBinaryVersion, latest_github_release};
#[cfg(not(target_os = "linux"))]
use http_client::github_download::download_server_binary;
use language::{LanguageServerName, LspAdapter, LspAdapterDelegate, LspInstaller, Toolchain};
use lsp::LanguageServerBinary;
use node_runtime::{NodeRuntime, VersionStrategy};
use serde_json::json;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    sync::Arc,
};
#[cfg(not(target_os = "linux"))]
use util::fs::{make_file_executable, remove_matching};
use util::{ResultExt, maybe};

pub struct SuperhtmlLspAdapter;

#[cfg(target_os = "macos")]
impl SuperhtmlLspAdapter {
    const GITHUB_ASSET_KIND: AssetKind = AssetKind::Zip;
    const OS_NAME: &str = "macos";
}

#[cfg(target_os = "windows")]
impl SuperhtmlLspAdapter {
    const GITHUB_ASSET_KIND: AssetKind = AssetKind::Zip;
    const OS_NAME: &str = "windows";
}

const SUPERHTML_SERVER_NAME: LanguageServerName = LanguageServerName::new_static("superhtml");

impl LspInstaller for SuperhtmlLspAdapter {
    type BinaryVersion = GitHubLspBinaryVersion;

    async fn check_if_user_installed(
        &self,
        delegate: &dyn LspAdapterDelegate,
        _: Option<Toolchain>,
        _: &AsyncApp,
    ) -> Option<LanguageServerBinary> {
        let path = delegate.which(with_exe("superhtml").as_ref()).await?;
        Some(LanguageServerBinary {
            path,
            arguments: vec!["lsp".into()],
            env: None,
        })
    }

    #[cfg(target_os = "linux")]
    async fn fetch_latest_server_version(
        &self,
        _delegate: &dyn LspAdapterDelegate,
        _pre_release: bool,
        _cx: &mut AsyncApp,
    ) -> Result<GitHubLspBinaryVersion> {
        anyhow::bail!("The superhtml language server has to be installed separately");
    }

    #[cfg(not(target_os = "linux"))]
    async fn fetch_latest_server_version(
        &self,
        delegate: &dyn LspAdapterDelegate,
        _pre_release: bool,
        _cx: &mut AsyncApp,
    ) -> Result<GitHubLspBinaryVersion> {
        let release = latest_github_release("kristoff-it/superhtml", false, false, delegate.http_client()).await?;

        let arch = match std::env::consts::ARCH {
            "aarch64" => "aarch64",
            "x86_64" => "x86_64",
            other => return Err(anyhow!("unsupported architecture: {}", other)),
        };

        let asset_name = format!(
            "{}-{}.{}",
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

    #[cfg(target_os = "linux")]
    async fn fetch_server_binary(
        &self,
        _version: GitHubLspBinaryVersion,
        _container_dir: PathBuf,
        _delegate: &dyn LspAdapterDelegate,
    ) -> Result<LanguageServerBinary> {
        anyhow::bail!("The superhtml language server has to be installed separately");
    }

    #[cfg(not(target_os = "linux"))]
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

        let destination_path = container_dir.join(format!("superhtml-{version_name}"));

        let server_path = destination_path.join(with_exe("superhtml"));

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
        match find_cached_server_binary(&container_dir, Some("superhtml-"), async |path| {
            Some(path.join(with_exe("superhtml")))
        })
        .await
        {
            Some(path) => Some(LanguageServerBinary {
                path,
                arguments: vec!["lsp".into()],
                env: None,
            }),
            None => None,
        }
    }
}

#[async_trait(?Send)]
impl LspAdapter for SuperhtmlLspAdapter {
    fn name(&self) -> LanguageServerName {
        SUPERHTML_SERVER_NAME
    }
}

const SERVER_PATH: &str = "node_modules/@zed-industries/vscode-langservers-extracted/bin/vscode-html-language-server";

fn server_binary_arguments(server_path: &Path) -> Vec<OsString> {
    vec![server_path.into(), "--stdio".into()]
}

pub struct HtmlLspAdapter {
    node: NodeRuntime,
}

impl HtmlLspAdapter {
    const PACKAGE_NAME: &str = "@zed-industries/vscode-langservers-extracted";

    pub fn new(node: NodeRuntime) -> Self {
        HtmlLspAdapter { node }
    }
}

impl LspInstaller for HtmlLspAdapter {
    type BinaryVersion = String;

    async fn fetch_latest_server_version(
        &self,
        _: &dyn LspAdapterDelegate,
        _: bool,
        _: &mut AsyncApp,
    ) -> Result<String> {
        self.node.npm_package_latest_version(Self::PACKAGE_NAME).await
    }

    async fn check_if_user_installed(
        &self,
        delegate: &dyn LspAdapterDelegate,
        _: Option<Toolchain>,
        _: &AsyncApp,
    ) -> Option<LanguageServerBinary> {
        let path = delegate.which(with_exe("vscode-html-language-server").as_ref()).await?;
        let env = delegate.shell_env().await;

        Some(LanguageServerBinary {
            path,
            env: Some(env),
            arguments: vec!["--stdio".into()],
        })
    }

    async fn fetch_server_binary(
        &self,
        latest_version: String,
        container_dir: PathBuf,
        _: &dyn LspAdapterDelegate,
    ) -> Result<LanguageServerBinary> {
        let server_path = container_dir.join(SERVER_PATH);

        self.node
            .npm_install_packages(&container_dir, &[(Self::PACKAGE_NAME, latest_version.as_str())])
            .await?;

        Ok(LanguageServerBinary {
            path: self.node.binary_path().await?,
            env: None,
            arguments: server_binary_arguments(&server_path),
        })
    }

    async fn check_if_version_installed(
        &self,
        version: &String,
        container_dir: &PathBuf,
        _: &dyn LspAdapterDelegate,
    ) -> Option<LanguageServerBinary> {
        let server_path = container_dir.join(SERVER_PATH);

        let should_install = self
            .node
            .should_install_npm_package(
                Self::PACKAGE_NAME,
                &server_path,
                container_dir,
                VersionStrategy::Latest(version),
            )
            .await;

        if should_install {
            None
        } else {
            Some(LanguageServerBinary {
                path: self.node.binary_path().await.ok()?,
                env: None,
                arguments: server_binary_arguments(&server_path),
            })
        }
    }

    async fn cached_server_binary(
        &self,
        container_dir: PathBuf,
        _: &dyn LspAdapterDelegate,
    ) -> Option<LanguageServerBinary> {
        get_cached_server_binary(container_dir, &self.node).await
    }
}

#[async_trait(?Send)]
impl LspAdapter for HtmlLspAdapter {
    fn name(&self) -> LanguageServerName {
        LanguageServerName("vscode-html-language-server".into())
    }

    async fn initialization_options(
        self: Arc<Self>,
        _: &Arc<dyn LspAdapterDelegate>,
    ) -> Result<Option<serde_json::Value>> {
        Ok(Some(json!({
            "provideFormatter": true
        })))
    }
}

async fn get_cached_server_binary(container_dir: PathBuf, node: &NodeRuntime) -> Option<LanguageServerBinary> {
    maybe!(async {
        let server_path = container_dir.join(SERVER_PATH);
        anyhow::ensure!(server_path.exists(), "missing executable in directory {server_path:?}");
        Ok(LanguageServerBinary {
            path: node.binary_path().await?,
            env: None,
            arguments: server_binary_arguments(&server_path),
        })
    })
    .await
    .log_err()
}
