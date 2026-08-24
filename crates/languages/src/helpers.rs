use anyhow::{Context, Result};
use http_client::github_download::GithubBinaryMetadata;
use language::LspAdapterDelegate;
use lsp::LanguageServerBinary;
use smol::{fs, stream::StreamExt};
use std::path::PathBuf;
use util::maybe;

pub fn with_exe(name: &str) -> String {
    let suffix = if cfg!(windows) { ".exe" } else { "" };
    format!("{}{}", name, suffix)
}

pub async fn write_metadata(destination_path: &PathBuf, expected_digest: Option<String>) -> Result<()> {
    let metadata_path = destination_path.with_extension("metadata");
    GithubBinaryMetadata::write_to_file(
        &GithubBinaryMetadata {
            metadata_version: 1,
            digest: expected_digest,
        },
        &metadata_path,
    )
    .await
}

pub async fn verify_metadata(
    destination_path: &PathBuf,
    server_path: &PathBuf,
    expected_digest: &Option<String>,
    delegate: &dyn LspAdapterDelegate,
) -> bool {
    let metadata_path = destination_path.with_extension("metadata");
    let metadata = GithubBinaryMetadata::read_from_file(&metadata_path).await.ok();
    let Some(metadata) = metadata else {
        return false;
    };

    let validity_check = async || {
        delegate
            .try_exec(LanguageServerBinary {
                path: server_path.clone(),
                arguments: vec!["--version".into()],
                env: None,
            })
            .await
            .inspect_err(|err| log::warn!("Unable to run {server_path:?} asset, redownloading: {err:#}",))
    };
    if let (Some(actual_digest), Some(expected_digest)) = (&metadata.digest, &expected_digest) {
        if actual_digest == expected_digest {
            if validity_check().await.is_ok() {
                return true;
            }
        } else {
            log::info!(
                "SHA-256 mismatch for {destination_path:?} asset, downloading new asset. Expected: {expected_digest}, Got: {actual_digest}"
            );
        }
    } else if validity_check().await.is_ok() {
        return true;
    }
    return false;
}

pub async fn find_cached_server_binary(
    container_dir: &PathBuf,
    prefix: Option<&str>,
    build_server_path: impl AsyncFn(&PathBuf) -> Option<PathBuf>,
) -> Option<PathBuf> {
    let binary_result = maybe!(async {
        let mut last = None;
        let mut entries = fs::read_dir(&container_dir)
            .await
            .with_context(|| format!("listing {container_dir:?}"))?;

        while let Some(entry) = entries.next().await {
            let Ok(entry) = entry else { continue };
            let path = entry.path();

            if path.extension().is_some_and(|ext| ext == "metadata") {
                continue;
            }

            let file_name = entry.file_name();
            let Some(file_name_str) = file_name.to_str() else {
                continue;
            };

            if let Some(prefix) = prefix {
                if !file_name_str.starts_with(prefix) {
                    continue;
                }
            }
            if let Some(server_path) = build_server_path(&path).await {
                if server_path.exists() {
                    log::info!("Cached LSP binary: {:?}", server_path);
                    last = Some(server_path);
                }
            }
        }
        anyhow::Ok(last)
    })
    .await;

    match binary_result {
        Ok(Some(binary)) => Some(binary),
        Ok(None) => {
            log::info!("No cached LSP binary found");
            None
        }
        Err(e) => {
            log::error!("Failed to look up cached LSP binary: {e:#}");
            None
        }
    }
}
