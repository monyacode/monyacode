/// LSP support for Odin
/// based on https://github.com/zed-extensions/odin
/// License: MIT
/// Author: Mo Nematzadeh
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use gpui::AsyncApp;
use http_client::github::AssetKind;
use http_client::github::{GitHubLspBinaryVersion, latest_github_release};
use http_client::github_download::download_server_binary;
pub use language::*;
use lsp::{CompletionItemKind, LanguageServerBinary, SymbolKind};
use project::ContextProviderWithTasks;
use std::path::PathBuf;
use std::sync::Arc;
use task::{TaskTemplate, TaskTemplates};
use util::fs::{make_file_executable, remove_matching};

use crate::helpers::{find_cached_server_binary, verify_metadata, with_exe, write_metadata};

pub struct OdinLspAdapter;

#[cfg(target_os = "macos")]
impl OdinLspAdapter {
    const GITHUB_ASSET_KIND: AssetKind = AssetKind::Zip;
    const OS_NAME: &str = "darwin";
}

#[cfg(target_os = "linux")]
impl OdinLspAdapter {
    const GITHUB_ASSET_KIND: AssetKind = AssetKind::Zip;
    const OS_NAME: &str = "unknown-linux-gnu";
}

#[cfg(target_os = "windows")]
impl OdinLspAdapter {
    const GITHUB_ASSET_KIND: AssetKind = AssetKind::Zip;
    const OS_NAME: &str = "pc-windows-msvc";
}

const SERVER_NAME: LanguageServerName = LanguageServerName::new_static("ols");

impl OdinLspAdapter {
    fn ols_binary_name() -> Option<String> {
        let arch = match std::env::consts::ARCH {
            "aarch64" => "arm64",
            "x86_64" => "x86_64",
            _ => return None, // Not supported
        };

        let os = Self::OS_NAME;
        let binary_name = format!("ols-{arch}-{os}");
        Some(binary_name)
    }

    async fn ols_path(path: &PathBuf) -> Option<PathBuf> {
        let binary_name = Self::ols_binary_name()?;
        let executable = with_exe(&binary_name);
        Some(path.join(executable))
    }
}

impl LspInstaller for OdinLspAdapter {
    type BinaryVersion = GitHubLspBinaryVersion;

    async fn check_if_user_installed(
        &self,
        delegate: &dyn LspAdapterDelegate,
        _: Option<Toolchain>,
        _: &AsyncApp,
    ) -> Option<LanguageServerBinary> {
        let path = delegate.which(with_exe("ols").as_ref()).await?;

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
        pre_release: bool,
        _cx: &mut AsyncApp,
    ) -> Result<GitHubLspBinaryVersion> {
        let release = latest_github_release("DanielGavin/ols", pre_release, false, delegate.http_client()).await?;

        let Some(binary_name) = Self::ols_binary_name() else {
            return Err(anyhow!("unsupported architecture: {}", std::env::consts::ARCH));
        };

        let asset_name = format!("{}.zip", binary_name);
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

        let destination_path = container_dir.join(format!("ols-{version_name}"));
        let server_path = match Self::GITHUB_ASSET_KIND {
            AssetKind::Zip => Self::ols_path(&destination_path)
                .await
                .ok_or_else(|| anyhow!("Unsupported architecture"))?,
            _ => unreachable!(),
        };

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
        match find_cached_server_binary(&container_dir, Some("ols-"), Self::ols_path).await {
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
impl LspAdapter for OdinLspAdapter {
    fn name(&self) -> LanguageServerName {
        SERVER_NAME
    }
    async fn label_for_completion(
        &self,
        completion: &lsp::CompletionItem,
        language: &Arc<Language>,
    ) -> Option<CodeLabel> {
        let completion = completion.clone();
        let kind = completion.kind?;
        let label = &completion.label;
        let filter_len = label.len();

        match kind {
            CompletionItemKind::STRUCT => {
                let code = match &completion.detail {
                    Some(detail) if detail.starts_with('[') || detail.starts_with("distinct") => {
                        format!("{} :: {}", label, detail)
                    }
                    _ => format!("{} :: struct", label),
                };
                Some(create_label(code, filter_len, language))
            }

            CompletionItemKind::ENUM => {
                let code = match &completion.detail {
                    // OLS sends union type info in detail field (e.g., "union { int, f32 }")
                    // We can detect and display it correctly here
                    Some(detail) if detail.contains("union") => {
                        format!("{} :: union", label)
                    }
                    Some(detail) if is_integer_type(detail) => {
                        format!("{} :: enum {}", label, detail)
                    }
                    _ => format!("{} :: enum", label),
                };
                Some(create_label(code, filter_len, language))
            }

            CompletionItemKind::VARIABLE | CompletionItemKind::FIELD => {
                let type_name = completion.detail.unwrap_or_else(|| "type".to_string());
                Some(create_label(format!("{}: {}", label, type_name), filter_len, language))
            }

            CompletionItemKind::CONSTANT => {
                let value = completion.detail.unwrap_or_else(|| "value".to_string());
                Some(create_label(format!("{} :: {}", label, value), filter_len, language))
            }

            CompletionItemKind::ENUM_MEMBER => {
                let code = format!(".{}", label);
                Some(create_label_with_span(code, 1..label.len() + 1, filter_len, language))
            }

            CompletionItemKind::PROPERTY => {
                let code = format!(".{}", label);
                Some(create_label_with_span(code, 1..label.len() + 1, filter_len, language))
            }

            CompletionItemKind::KEYWORD => {
                let source = Rope::from_iter([label, "keyword"]);
                let runs = language.highlight_text(&source, 0..label.len());
                let text = label.clone();
                let filter_range = 0..filter_len;
                Some(CodeLabel::new(text, filter_range, runs))
            }

            CompletionItemKind::MODULE => {
                let code = format!("package {}", label);
                Some(create_label_with_span(code, 8..label.len() + 8, filter_len, language))
            }

            _ => None,
        }
    }

    async fn label_for_symbol(&self, name: &str, kind: SymbolKind, language: &Arc<Language>) -> Option<CodeLabel> {
        // NOTE: Symbol navigation has limited type information compared to completions.
        // The LSP Symbol type only provides 'name' and 'kind', without detailed type info.

        let filter_len = name.len();

        match kind {
            SymbolKind::FUNCTION => Some(create_label(format!("{} :: proc", name), filter_len, language)),
            SymbolKind::VARIABLE => Some(create_label(format!("{}: type", name), filter_len, language)),
            SymbolKind::STRUCT => Some(create_label(format!("{} :: struct", name), filter_len, language)),
            // OLS sends both enums and unions as Enum kind (cannot distinguish in symbols)
            SymbolKind::ENUM => Some(create_label(format!("{} :: enum", name), filter_len, language)),
            // Struct and union fields
            SymbolKind::FIELD => Some(create_label(format!("{}: type", name), filter_len, language)),
            _ => None,
        }
    }
}

pub(super) fn odin_task_context() -> ContextProviderWithTasks {
    ContextProviderWithTasks::new(TaskTemplates(vec![
        TaskTemplate {
            label: "odin run .".into(),
            command: "odin".into(),
            args: vec!["run".into(), ".".into()],
            tags: vec!["odin-main".into()],
            ..Default::default()
        },
        TaskTemplate {
            label: "odin test .".into(),
            command: "odin".into(),
            args: vec!["test".into(), ".".into()],
            tags: vec!["odin-test".into()],
            ..Default::default()
        },
    ]))
}

fn is_integer_type(type_str: &str) -> bool {
    matches!(
        type_str,
        // Basic signed integers
        "int" | "i8" | "i16" | "i32" | "i64" | "i128" |
        // Basic unsigned integers
        "uint" | "u8" | "u16" | "u32" | "u64" | "u128" | "uintptr" |
        // Integer aliases
        "byte" | "rune" |
        // Little-endian integers
        "i16le" | "i32le" | "i64le" | "i128le" |
        "u16le" | "u32le" | "u64le" | "u128le" |
        // Big-endian integers
        "i16be" | "i32be" | "i64be" | "i128be" |
        "u16be" | "u32be" | "u64be" | "u128be"
    )
}

fn create_label(code: String, filter_len: usize, language: &Arc<Language>) -> CodeLabel {
    let source = Rope::from(&code);
    let len = code.len();
    let runs = language.highlight_text(&source, 0..len);
    CodeLabel::new(code, 0..filter_len, runs)
}

fn create_label_with_span(
    code: String,
    span_range: std::ops::Range<usize>,
    filter_len: usize,
    language: &Arc<Language>,
) -> CodeLabel {
    let source = Rope::from(&code);
    let runs = language.highlight_text(&source, span_range);
    CodeLabel::new(code, 0..filter_len, runs)
}
