/// LSP support for Nix
/// based on https://github.com/zed-extensions/nix
/// License: MIT
/// Author: Hasit Mistry
use anyhow::Result;
use async_trait::async_trait;
use gpui::AsyncApp;
pub use language::*;
use lsp::LanguageServerBinary;
use std::path::PathBuf;

use crate::helpers::with_exe;

pub struct NilLspAdapter;
pub struct NixdLspAdapter;

const NIL_NAME: LanguageServerName = LanguageServerName::new_static("nil");
const NIXD_NAME: LanguageServerName = LanguageServerName::new_static("nixd");

impl LspInstaller for NilLspAdapter {
    type BinaryVersion = String;

    async fn check_if_user_installed(
        &self,
        delegate: &dyn LspAdapterDelegate,
        _: Option<Toolchain>,
        _: &AsyncApp,
    ) -> Option<LanguageServerBinary> {
        let path = delegate.which(with_exe("nil").as_ref()).await?;
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
    ) -> Result<String> {
        anyhow::bail!("The nil language server has to be installed separately")
    }

    async fn fetch_server_binary(
        &self,
        _version: String,
        _container_dir: PathBuf,
        _delegate: &dyn LspAdapterDelegate,
    ) -> Result<LanguageServerBinary> {
        anyhow::bail!("The nil language server has to be installed separately")
    }

    async fn cached_server_binary(
        &self,
        _container_dir: PathBuf,
        _: &dyn LspAdapterDelegate,
    ) -> Option<LanguageServerBinary> {
        None
    }
}

#[async_trait(?Send)]
impl LspAdapter for NilLspAdapter {
    fn name(&self) -> LanguageServerName {
        NIL_NAME
    }
}

impl LspInstaller for NixdLspAdapter {
    type BinaryVersion = String;

    async fn check_if_user_installed(
        &self,
        delegate: &dyn LspAdapterDelegate,
        _: Option<Toolchain>,
        _: &AsyncApp,
    ) -> Option<LanguageServerBinary> {
        let path = delegate.which(with_exe("nixd").as_ref()).await?;
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
    ) -> Result<String> {
        anyhow::bail!("The nixd language server has to be installed separately")
    }

    async fn fetch_server_binary(
        &self,
        _version: String,
        _container_dir: PathBuf,
        _delegate: &dyn LspAdapterDelegate,
    ) -> Result<LanguageServerBinary> {
        anyhow::bail!("The nixd language server has to be installed separately")
    }

    async fn cached_server_binary(
        &self,
        _container_dir: PathBuf,
        _: &dyn LspAdapterDelegate,
    ) -> Option<LanguageServerBinary> {
        None
    }
}

#[async_trait(?Send)]
impl LspAdapter for NixdLspAdapter {
    fn name(&self) -> LanguageServerName {
        NIXD_NAME
    }
}
