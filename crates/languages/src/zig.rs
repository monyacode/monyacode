/// LSP support for Zig
/// based on https://github.com/zed-extensions/zig
/// License: ./LICENSE-APACHE
/// Author: Allan Calix
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use gpui::AsyncApp;
use http_client::github::AssetKind;
use http_client::github::latest_github_release;
use http_client::github_download::download_server_binary;
pub use language::*;
use lsp::LanguageServerBinary;
use project::ContextProviderWithTasks;
use std::path::PathBuf;
use task::{TaskTemplate, TaskTemplates};
use util::fs::{make_file_executable, remove_matching};

use crate::helpers::find_cached_server_binary;
use crate::helpers::verify_metadata;
use crate::helpers::with_exe;
use crate::helpers::write_metadata;

#[derive(Clone, Debug)]
pub struct ZlsBinaryVersion {
    version: String,
    download_url: String,
}

pub struct ZigLspAdapter;

#[cfg(target_os = "macos")]
impl ZigLspAdapter {
    const ARCHIVE_TYPE: AssetKind = AssetKind::TarGz;
    const OS_NAME: &str = "macos";
}

#[cfg(target_os = "linux")]
impl ZigLspAdapter {
    const ARCHIVE_TYPE: AssetKind = AssetKind::TarGz;
    const OS_NAME: &str = "linux";
}

#[cfg(target_os = "windows")]
impl ZigLspAdapter {
    const ARCHIVE_TYPE: AssetKind = AssetKind::Zip;
    const OS_NAME: &str = "windows";
}

const SERVER_NAME: LanguageServerName = LanguageServerName::new_static("zls");

impl LspInstaller for ZigLspAdapter {
    type BinaryVersion = ZlsBinaryVersion;

    async fn check_if_user_installed(
        &self,
        delegate: &dyn LspAdapterDelegate,
        _: Option<Toolchain>,
        _: &AsyncApp,
    ) -> Option<LanguageServerBinary> {
        let path = delegate.which(with_exe("zls").as_ref()).await?;

        #[cfg(unix)]
        let env = Some(delegate.shell_env().await);
        #[cfg(not(unix))]
        let env = None;

        Some(LanguageServerBinary {
            path,
            arguments: vec![],
            env,
        })
    }

    async fn fetch_latest_server_version(
        &self,
        delegate: &dyn LspAdapterDelegate,
        _pre_release: bool,
        _cx: &mut AsyncApp,
    ) -> Result<ZlsBinaryVersion> {
        // Note that in github releases and on zlstools.org the tar.gz asset is not shown
        // but is available at https://builds.zigtools.org/zls-{os}-{arch}-{version}.tar.gz
        let release = latest_github_release("zigtools/zls", true, false, delegate.http_client()).await?;

        let arch = match std::env::consts::ARCH {
            "aarch64" => "aarch64",
            "x86" => "x86",
            "x86_64" => "x86_64",
            other => return Err(anyhow!("unsupported architecture: {}", other)),
        };

        let extension = match Self::ARCHIVE_TYPE {
            AssetKind::TarGz => "tar.gz",
            AssetKind::Zip => "zip",
            _ => unreachable!(),
        };

        let asset_name = format!("zls-{}-{}-{}.{}", arch, Self::OS_NAME, release.tag_name, extension);
        let download_url = format!("https://builds.zigtools.org/{}", asset_name);

        Ok(ZlsBinaryVersion {
            version: release.tag_name,
            download_url,
        })
    }

    async fn fetch_server_binary(
        &self,
        version: ZlsBinaryVersion,
        container_dir: PathBuf,
        delegate: &dyn LspAdapterDelegate,
    ) -> Result<LanguageServerBinary> {
        log::info!(
            "fetch_server_binary: version={:?} dir={:?}",
            version.version,
            container_dir
        );

        let ZlsBinaryVersion {
            version: version_name,
            download_url,
        } = version;

        let destination_path = container_dir.join(format!("zls-{version_name}"));
        let server_path = match Self::ARCHIVE_TYPE {
            AssetKind::TarGz => destination_path.join("zls"),
            AssetKind::Zip => destination_path.join("zls.exe"),
            _ => unreachable!(),
        };

        let binary = LanguageServerBinary {
            path: server_path.clone(),
            env: None,
            arguments: vec![],
        };

        if verify_metadata(&destination_path, &server_path, &None, delegate).await {
            return Ok(binary);
        }
        download_server_binary(
            &*delegate.http_client(),
            &download_url,
            None,
            &destination_path,
            Self::ARCHIVE_TYPE,
        )
        .await?;
        make_file_executable(&server_path).await?;
        remove_matching(&container_dir, |path| path != destination_path).await;
        write_metadata(&destination_path, None).await?;
        Ok(binary)
    }

    async fn cached_server_binary(
        &self,
        container_dir: PathBuf,
        _: &dyn LspAdapterDelegate,
    ) -> Option<LanguageServerBinary> {
        match find_cached_server_binary(&container_dir, Some("zls-"), async |path| {
            Some(path.join(with_exe("zls")))
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
impl LspAdapter for ZigLspAdapter {
    fn name(&self) -> LanguageServerName {
        SERVER_NAME
    }
}

pub(super) fn zig_task_context() -> ContextProviderWithTasks {
    ContextProviderWithTasks::new(TaskTemplates(vec![
        TaskTemplate {
            label: "zig build run".into(),
            command: "zig".into(),
            args: vec!["build".into(), "run".into()],
            tags: vec!["zig-build-run".into()],
            ..Default::default()
        },
        TaskTemplate {
            label: "zig build test".into(),
            command: "zig".into(),
            args: vec!["build".into(), "test".into()],
            ..Default::default()
        },
    ]))
}
