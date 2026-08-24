mod capability_granter;
pub mod extension_settings;
pub mod headless_host;
pub mod wasm_host;

use anyhow::{Context as _, Result, anyhow, bail};
use client::{Client, ExtensionMetadata, proto};
use collections::{BTreeMap, HashMap, HashSet, btree_map};
pub use extension::ExtensionManifest;
use extension::extension_builder::{CompileExtensionOptions, ExtensionBuilder};
use extension::{
    ExtensionDebugAdapterProviderProxy, ExtensionEvents, ExtensionHostProxy, ExtensionLanguageProxy,
    ExtensionLanguageServerProxy, ExtensionGrammarProxy, ExtensionSnippetProxy, ExtensionThemeProxy,
};
use fs::{Fs, RemoveOptions};
use futures::future::join_all;
use futures::{
    Future, FutureExt as _, StreamExt as _,
    channel::{
        mpsc::{UnboundedSender, unbounded},
        oneshot,
    },
    select_biased,
};
use gpui::{App, AppContext as _, AsyncApp, Context, Entity, EventEmitter, Global, Task, WeakEntity, actions};
use http_client::{HttpClient, HttpClientWithUrl};
use language::{
    LanguageConfig, LanguageMatcher, LanguageName, LanguageQueries, LoadedLanguage, QUERY_FILENAME_PREFIXES, Rope,
};
use node_runtime::NodeRuntime;
use project::ContextProviderWithTasks;
use release_channel::ReleaseChannel;
use remote::RemoteClient;
use semver::Version as SemanticVersion;
use serde::{Deserialize, Serialize};
use std::fs::{copy, create_dir_all, read_dir};
use std::ops::RangeInclusive;
use std::str::FromStr;
use std::{
    cmp::Ordering,
    path::{self, Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use util::{ResultExt, paths::RemotePathBuf};
use wasm_host::{WasmExtension, WasmHost, wit::is_supported_wasm_api_version};

pub use extension::{ExtensionLibraryKind, GrammarManifestEntry, OldExtensionManifest, SchemaVersion};
pub use extension_settings::ExtensionSettings;

pub const RELOAD_DEBOUNCE_DURATION: Duration = Duration::from_millis(200);
const FS_WATCH_LATENCY: Duration = Duration::from_millis(100);

/// The current extension [`SchemaVersion`] supported by MonyaCode.
const CURRENT_SCHEMA_VERSION: SchemaVersion = SchemaVersion(1);

/// Extensions that should no longer be loaded or downloaded.
///
/// These snippets should no longer be downloaded or loaded, because their
/// functionality has been integrated into the core editor.
const SUPPRESSED_EXTENSIONS: &[&str] = &["snippets", "ruff", "ty", "basedpyright"];

/// Returns the [`SchemaVersion`] range that is compatible with this version of MonyaCode.
pub fn schema_version_range() -> RangeInclusive<SchemaVersion> {
    SchemaVersion::ZERO..=CURRENT_SCHEMA_VERSION
}

/// Returns whether the given extension version is compatible with this version of MonyaCode.
pub fn is_version_compatible(release_channel: ReleaseChannel, extension_version: &ExtensionMetadata) -> bool {
    let schema_version = extension_version.manifest.schema_version.unwrap_or(0);
    if CURRENT_SCHEMA_VERSION.0 < schema_version {
        return false;
    }

    if let Some(wasm_api_version) = extension_version
        .manifest
        .wasm_api_version
        .as_ref()
        .and_then(|wasm_api_version| SemanticVersion::from_str(wasm_api_version).ok())
        && !is_supported_wasm_api_version(release_channel, wasm_api_version)
    {
        return false;
    }

    true
}

pub struct ExtensionStore {
    pub proxy: Arc<ExtensionHostProxy>,
    pub builder: Arc<ExtensionBuilder>,
    pub extension_index: ExtensionIndex,
    pub fs: Arc<dyn Fs>,
    pub http_client: Arc<HttpClientWithUrl>,
    pub reload_tx: UnboundedSender<Option<Arc<str>>>,
    pub reload_complete_senders: Vec<oneshot::Sender<()>>,
    pub installed_dir: PathBuf,
    pub outstanding_operations: BTreeMap<Arc<str>, ExtensionOperation>,
    pub index_path: PathBuf,
    pub modified_extensions: HashSet<Arc<str>>,
    pub wasm_host: Arc<WasmHost>,
    pub wasm_extensions: Vec<(Arc<ExtensionManifest>, WasmExtension)>,
    pub tasks: Vec<Task<()>>,
    pub remote_clients: Vec<WeakEntity<RemoteClient>>,
    pub ssh_registered_tx: UnboundedSender<()>,
}

#[derive(Clone, Copy)]
pub enum ExtensionOperation {
    Upgrade,
    Install,
    Remove,
}

#[derive(Clone)]
pub enum Event {
    ExtensionsUpdated,
    StartedReloading,
    ExtensionInstalled(Arc<str>),
    ExtensionUninstalled(Arc<str>),
    ExtensionFailedToLoad(Arc<str>),
}

impl EventEmitter<Event> for ExtensionStore {}

struct GlobalExtensionStore(Entity<ExtensionStore>);

impl Global for GlobalExtensionStore {}

#[derive(Debug, Deserialize, Serialize, Default, PartialEq, Eq)]
pub struct ExtensionIndex {
    pub extensions: BTreeMap<Arc<str>, ExtensionIndexEntry>,
    pub themes: BTreeMap<Arc<str>, ExtensionIndexThemeEntry>,
    #[serde(default)]
    pub icon_themes: BTreeMap<Arc<str>, ExtensionIndexIconThemeEntry>,
    pub languages: BTreeMap<LanguageName, ExtensionIndexLanguageEntry>,
}

impl ExtensionIndex {
    fn extensions_to_sync_to_remote(&self) -> RemoteSyncExtensions {
        let mut extensions = RemoteSyncExtensions::default();

        for (id, entry) in &self.extensions {
            if entry.manifest.remote_load().is_some() {
                extensions.insert_extension_and_language_dependencies(self, id);
            }
        }

        extensions
    }
}

#[derive(Default)]
struct RemoteSyncExtensions(HashMap<Arc<str>, ExtensionIndexEntry>);

impl RemoteSyncExtensions {
    fn insert_extension_and_language_dependencies(&mut self, index: &ExtensionIndex, id: &Arc<str>) {
        if self.0.contains_key(id) {
            return;
        }

        let Some(entry) = index.extensions.get(id) else {
            return;
        };

        self.0.insert(id.clone(), entry.clone());

        let Some(remote_load) = entry.manifest.remote_load() else {
            return;
        };

        for language in remote_load.language_dependencies() {
            if let Some(language_entry) = index.languages.get(&language) {
                self.insert_extension_and_language_dependencies(index, &language_entry.extension);
            }
        }
    }

    fn into_entries(self) -> impl Iterator<Item = (Arc<str>, ExtensionIndexEntry)> {
        self.0.into_iter()
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Deserialize, Serialize)]
pub struct ExtensionIndexEntry {
    pub manifest: Arc<ExtensionManifest>,
    pub dev: bool,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct ExtensionIndexThemeEntry {
    pub extension: Arc<str>,
    pub path: PathBuf,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct ExtensionIndexIconThemeEntry {
    pub extension: Arc<str>,
    pub path: PathBuf,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug, Deserialize, Serialize)]
pub struct ExtensionIndexLanguageEntry {
    pub extension: Arc<str>,
    pub path: PathBuf,
    pub matcher: LanguageMatcher,
    pub hidden: bool,
    pub grammar: Option<Arc<str>>,
}

actions!(
    monyacode,
    [
        /// Reloads all installed extensions.
        ReloadExtensions
    ]
);

pub fn init(
    extension_host_proxy: Arc<ExtensionHostProxy>,
    fs: Arc<dyn Fs>,
    client: Arc<Client>,
    node_runtime: NodeRuntime,
    cx: &mut App,
) {
    let store = cx.new(move |cx| {
        ExtensionStore::new(
            paths::extensions_dir().clone(),
            None,
            extension_host_proxy,
            fs,
            client.http_client(),
            client.http_client(),
            node_runtime,
            cx,
        )
    });

    cx.on_action(|_: &ReloadExtensions, cx| {
        let store = cx.global::<GlobalExtensionStore>().0.clone();
        store.update(cx, |store, cx| drop(store.reload(None, cx)));
    });

    cx.set_global(GlobalExtensionStore(store));
}

impl ExtensionStore {
    pub fn try_global(cx: &App) -> Option<Entity<Self>> {
        cx.try_global::<GlobalExtensionStore>().map(|store| store.0.clone())
    }

    pub fn global(cx: &App) -> Entity<Self> {
        cx.global::<GlobalExtensionStore>().0.clone()
    }

    pub fn new(
        extensions_dir: PathBuf,
        build_dir: Option<PathBuf>,
        extension_host_proxy: Arc<ExtensionHostProxy>,
        fs: Arc<dyn Fs>,
        http_client: Arc<HttpClientWithUrl>,
        builder_client: Arc<dyn HttpClient>,
        node_runtime: NodeRuntime,
        cx: &mut Context<Self>,
    ) -> Self {
        let work_dir = extensions_dir.join("work");
        let build_dir = build_dir.unwrap_or_else(|| extensions_dir.join("build"));
        let installed_dir = extensions_dir.join("installed");
        let index_path = extensions_dir.join("index.json");

        let (reload_tx, mut reload_rx) = unbounded();
        let (connection_registered_tx, mut connection_registered_rx) = unbounded();
        let mut this = Self {
            proxy: extension_host_proxy.clone(),
            extension_index: Default::default(),
            installed_dir,
            index_path,
            builder: Arc::new(ExtensionBuilder::new(builder_client, build_dir)),
            outstanding_operations: Default::default(),
            modified_extensions: Default::default(),
            reload_complete_senders: Vec::new(),
            wasm_host: WasmHost::new(
                fs.clone(),
                http_client.clone(),
                node_runtime,
                extension_host_proxy,
                work_dir,
                cx,
            ),
            wasm_extensions: Vec::new(),
            fs,
            http_client,
            reload_tx,
            tasks: Vec::new(),

            remote_clients: Default::default(),
            ssh_registered_tx: connection_registered_tx,
        };

        // The extensions store maintains an index file, which contains a complete
        // list of the installed extensions and the resources that they provide.
        // This index is loaded synchronously on startup.
        let (index_content, index_metadata, extensions_metadata) = cx.background_executor().block(async {
            futures::join!(
                this.fs.load(&this.index_path),
                this.fs.metadata(&this.index_path),
                this.fs.metadata(&this.installed_dir),
            )
        });

        // Normally, there is no need to rebuild the index. But if the index file
        // is invalid or is out-of-date according to the filesystem mtimes, then
        // it must be asynchronously rebuilt.
        let mut extension_index = ExtensionIndex::default();
        let mut extension_index_needs_rebuild = true;
        if let Ok(index_content) = index_content
            && let Some(index) = serde_json::from_str(&index_content).log_err()
        {
            extension_index = index;
            if let (Ok(Some(index_metadata)), Ok(Some(extensions_metadata))) = (index_metadata, extensions_metadata)
                && index_metadata.mtime.bad_is_greater_than(extensions_metadata.mtime)
            {
                extension_index_needs_rebuild = false;
            }
        }

        // Immediately load all of the extensions in the initial manifest. If the
        // index needs to be rebuild, then enqueue
        let load_initial_extensions = this.extensions_updated(extension_index, cx);
        let mut reload_future = None;
        if extension_index_needs_rebuild {
            log::info!("Rebuild the index!");
            reload_future = Some(this.reload(None, cx));
        }

        cx.spawn(async move |this, cx| {
            if let Some(future) = reload_future {
                future.await;
            }
            this.update(cx, |this, cx| this.auto_install_extensions(cx)).ok();
            // this.update(cx, |this, cx| this.check_for_updates(cx)).ok();
        })
        .detach();

        // Perform all extension loading in a single task to ensure that we
        // never attempt to simultaneously load/unload extensions from multiple
        // parallel tasks.
        this.tasks.push(cx.spawn(async move |this, cx| {
            async move {
                load_initial_extensions.await;

                let mut index_changed = true;
                let mut debounce_timer = cx.background_spawn(futures::future::pending()).fuse();
                loop {
                    select_biased! {
                        _ = debounce_timer => {
                            if index_changed {
                                let index = this
                                    .update(cx, |this, cx| this.rebuild_extension_index(cx))?
                                    .await;
                                this.update(cx, |this, cx| this.extensions_updated(index, cx))?
                                    .await;
                                index_changed = false;
                            }

                            Self::update_remote_clients(&this, cx).await?;
                        }
                        _ = connection_registered_rx.next() => {
                            debounce_timer = cx
                                .background_executor()
                                .timer(RELOAD_DEBOUNCE_DURATION)
                                .fuse();
                        }
                        extension_id = reload_rx.next() => {
                            let Some(extension_id) = extension_id else { break; };
                            this.update(cx, |this, _| {
                                this.modified_extensions.extend(extension_id);
                            })?;
                            index_changed = true;
                            debounce_timer = cx
                                .background_executor()
                                .timer(RELOAD_DEBOUNCE_DURATION)
                                .fuse();
                        }
                    }
                }

                anyhow::Ok(())
            }
            .map(drop)
            .await;
        }));

        // Watch the installed extensions directory for changes. Whenever changes are
        // detected, rebuild the extension index, and load/unload any extensions that
        // have been added, removed, or modified.
        this.tasks.push(cx.background_spawn({
            let fs = this.fs.clone();
            let reload_tx = this.reload_tx.clone();
            let installed_dir = this.installed_dir.clone();
            async move {
                let (mut paths, _) = fs
                    .watch(&installed_dir, FS_WATCH_LATENCY, fs::fs_watcher::WatcherMode::Native)
                    .await;
                while let Some(events) = paths.next().await {
                    for event in events {
                        let Ok(event_path) = event.path.strip_prefix(&installed_dir) else {
                            continue;
                        };

                        if let Some(path::Component::Normal(extension_dir_name)) = event_path.components().next()
                            && let Some(extension_id) = extension_dir_name.to_str()
                        {
                            reload_tx.unbounded_send(Some(extension_id.into())).ok();
                        }
                    }
                }
            }
        }));

        this
    }

    pub fn reload(
        &mut self,
        modified_extension: Option<Arc<str>>,
        cx: &mut Context<Self>,
    ) -> impl Future<Output = ()> + use<> {
        let (tx, rx) = oneshot::channel();
        self.reload_complete_senders.push(tx);
        self.reload_tx
            .unbounded_send(modified_extension)
            .expect("reload task exited");
        cx.emit(Event::StartedReloading);

        async move {
            rx.await.ok();
        }
    }

    /// We don't auto-install any extensions at the moment
    /// (since we don't want to download anything without
    /// explicit permission or maintain any central repository)
    ///
    /// However, I suspect the auto-installation is how all of
    /// the directories we need are created in Zed...
    pub fn auto_install_extensions(&mut self, _cx: &mut Context<Self>) {}

    fn extensions_dir(&self) -> PathBuf {
        self.installed_dir.clone()
    }

    pub fn outstanding_operations(&self) -> &BTreeMap<Arc<str>, ExtensionOperation> {
        &self.outstanding_operations
    }

    pub fn installed_extensions(&self) -> &BTreeMap<Arc<str>, ExtensionIndexEntry> {
        &self.extension_index.extensions
    }

    pub fn dev_extensions(&self) -> impl Iterator<Item = &Arc<ExtensionManifest>> {
        self.extension_index
            .extensions
            .values()
            .filter_map(|extension| extension.dev.then_some(&extension.manifest))
    }

    pub fn extension_manifest_for_id(&self, extension_id: &str) -> Option<&Arc<ExtensionManifest>> {
        self.extension_index
            .extensions
            .get(extension_id)
            .map(|extension| &extension.manifest)
    }

    /// Returns the names of themes provided by extensions.
    pub fn extension_themes<'a>(&'a self, extension_id: &'a str) -> impl Iterator<Item = &'a Arc<str>> {
        self.extension_index
            .themes
            .iter()
            .filter_map(|(name, theme)| theme.extension.as_ref().eq(extension_id).then_some(name))
    }

    /// Returns the path to the theme file within an extension, if there is an
    /// extension that provides the theme.
    pub fn path_to_extension_theme(&self, theme_name: &str) -> Option<PathBuf> {
        let entry = self.extension_index.themes.get(theme_name)?;

        Some(self.extensions_dir().join(entry.extension.as_ref()).join(&entry.path))
    }

    /// Returns the names of icon themes provided by extensions.
    pub fn extension_icon_themes<'a>(&'a self, extension_id: &'a str) -> impl Iterator<Item = &'a Arc<str>> {
        self.extension_index
            .icon_themes
            .iter()
            .filter_map(|(name, icon_theme)| icon_theme.extension.as_ref().eq(extension_id).then_some(name))
    }

    /// Returns the path to the icon theme file within an extension, if there is
    /// an extension that provides the icon theme.
    pub fn path_to_extension_icon_theme(&self, icon_theme_name: &str) -> Option<(PathBuf, PathBuf)> {
        let entry = self.extension_index.icon_themes.get(icon_theme_name)?;

        let icon_theme_path = self.extensions_dir().join(entry.extension.as_ref()).join(&entry.path);
        let icons_root_path = self.extensions_dir().join(entry.extension.as_ref());

        Some((icon_theme_path, icons_root_path))
    }

    pub fn uninstall_extension(&mut self, extension_id: Arc<str>, cx: &mut Context<Self>) -> Task<Result<()>> {
        let extension_dir = self.installed_dir.join(extension_id.as_ref());
        let work_dir = self.wasm_host.work_dir.join(extension_id.as_ref());
        let fs = self.fs.clone();

        let extension_manifest = self.extension_manifest_for_id(&extension_id).cloned();

        match self.outstanding_operations.entry(extension_id.clone()) {
            btree_map::Entry::Occupied(_) => return Task::ready(Ok(())),
            btree_map::Entry::Vacant(e) => e.insert(ExtensionOperation::Remove),
        };

        cx.spawn(async move |extension_store, cx| {
            let _finish = cx.on_drop(&extension_store, {
                let extension_id = extension_id.clone();
                move |this, cx| {
                    this.outstanding_operations.remove(extension_id.as_ref());
                    cx.notify();
                }
            });

            fs.remove_dir(
                &extension_dir,
                RemoveOptions {
                    recursive: true,
                    ignore_if_not_exists: true,
                },
            )
            .await
            .with_context(|| format!("Removing extension dir {extension_dir:?}"))?;

            extension_store
                .update(cx, |extension_store, cx| extension_store.reload(None, cx))?
                .await;

            // There's a race between wasm extension fully stopping and the directory removal.
            // On Windows, it's impossible to remove a directory that has a process running in it.
            for i in 0..3 {
                cx.background_executor().timer(Duration::from_millis(i * 100)).await;
                let removal_result = fs
                    .remove_dir(
                        &work_dir,
                        RemoveOptions {
                            recursive: true,
                            ignore_if_not_exists: true,
                        },
                    )
                    .await;
                match removal_result {
                    Ok(()) => break,
                    Err(e) => {
                        if i == 2 {
                            log::error!("Failed to remove extension work dir {work_dir:?} : {e}");
                        }
                    }
                }
            }

            extension_store.update(cx, |_, cx| {
                cx.emit(Event::ExtensionUninstalled(extension_id.clone()));
                if let Some(events) = ExtensionEvents::try_global(cx)
                    && let Some(manifest) = extension_manifest
                {
                    events.update(cx, |this, cx| {
                        this.emit(extension::Event::ExtensionUninstalled(manifest.clone()), cx)
                    });
                }
            })?;

            anyhow::Ok(())
        })
    }

    pub fn install_dev_extension(
        &mut self,
        extension_source_path: PathBuf,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        let extensions_dir = self.extensions_dir();
        let fs = self.fs.clone();
        let builder = self.builder.clone();

        cx.spawn(async move |this, cx| {
            let mut extension_manifest = ExtensionManifest::load(fs.clone(), &extension_source_path).await?;
            let extension_id = extension_manifest.id.clone();

            if let Some(uninstall_task) = this
                .update(cx, |this, cx| {
                    this.extension_index
                        .extensions
                        .get(extension_id.as_ref())
                        .is_some_and(|index_entry| !index_entry.dev)
                        .then(|| this.uninstall_extension(extension_id.clone(), cx))
                })
                .ok()
                .flatten()
            {
                uninstall_task.await.log_err();
            }

            if !this.update(cx, |this, cx| {
                match this.outstanding_operations.entry(extension_id.clone()) {
                    btree_map::Entry::Occupied(_) => return false,
                    btree_map::Entry::Vacant(e) => e.insert(ExtensionOperation::Install),
                };
                cx.notify();
                true
            })? {
                return Ok(());
            }

            let _finish = cx.on_drop(&this, {
                let extension_id = extension_id.clone();
                move |this, cx| {
                    this.outstanding_operations.remove(extension_id.as_ref());
                    cx.notify();
                }
            });

            cx.background_spawn({
                let extension_source_path = extension_source_path.clone();
                let fs = fs.clone();
                async move {
                    builder
                        .compile_extension(
                            &extension_source_path,
                            &mut extension_manifest,
                            CompileExtensionOptions { release: true },
                            fs,
                        )
                        .await
                }
            })
            .await
            .inspect_err(|error| {
                util::log_err(error);
            })?;

            log::info!("Compiled {:?} to {:?}", extension_id, extensions_dir);

            let output_path = &extensions_dir.join(extension_id.as_ref());
            log::info!("Now this should exist.. {}", output_path.display());
            if let Some(metadata) = fs.metadata(output_path).await? {
                if metadata.is_symlink {
                    fs.remove_file(
                        output_path,
                        RemoveOptions {
                            recursive: false,
                            ignore_if_not_exists: true,
                        },
                    )
                    .await?;
                } else {
                    bail!("extension {extension_id} is still installed");
                }
            }

            log::info!(
                "copy_dir from={}, to={}",
                extension_source_path.display(),
                output_path.display()
            );
            copy_dir(extension_source_path.as_path(), output_path.as_path()).await?;

            this.update(cx, |this, cx| this.reload(None, cx))?.await;
            this.update(cx, |this, cx| {
                cx.emit(Event::ExtensionInstalled(extension_id.clone()));
                if let Some(events) = ExtensionEvents::try_global(cx)
                    && let Some(manifest) = this.extension_manifest_for_id(&extension_id)
                {
                    events.update(cx, |this, cx| {
                        this.emit(extension::Event::ExtensionInstalled(manifest.clone()), cx)
                    });
                }
            })?;

            Ok(())
        })
    }

    pub fn rebuild_dev_extension(&mut self, extension_id: Arc<str>, cx: &mut Context<Self>) {
        let path = self.installed_dir.join(extension_id.as_ref());
        let builder = self.builder.clone();
        let fs = self.fs.clone();

        match self.outstanding_operations.entry(extension_id.clone()) {
            btree_map::Entry::Occupied(_) => return,
            btree_map::Entry::Vacant(e) => e.insert(ExtensionOperation::Upgrade),
        };

        cx.notify();
        let compile = cx.background_spawn(async move {
            let mut manifest = ExtensionManifest::load(fs.clone(), &path).await?;
            builder
                .compile_extension(&path, &mut manifest, CompileExtensionOptions { release: true }, fs)
                .await
        });

        cx.spawn(async move |this, cx| {
            let result = compile.await;

            this.update(cx, |this, cx| {
                this.outstanding_operations.remove(&extension_id);
                cx.notify();
            })?;

            if result.is_ok() {
                this.update(cx, |this, cx| this.reload(Some(extension_id), cx))?.await;
            }

            result
        })
        .detach_and_log_err(cx)
    }

    /// Updates the set of installed extensions.
    ///
    /// First, this unloads any themes, languages, or grammars that are
    /// no longer in the manifest, or whose files have changed on disk.
    /// Then it loads any themes, languages, or grammars that are newly
    /// added to the manifest, or whose files have changed on disk.
    fn extensions_updated(&mut self, mut new_index: ExtensionIndex, cx: &mut Context<Self>) -> Task<()> {
        let old_index = &self.extension_index;

        new_index
            .extensions
            .retain(|extension_id, _| !SUPPRESSED_EXTENSIONS.contains(&extension_id.as_ref()));

        // Determine which extensions need to be loaded and unloaded, based
        // on the changes to the manifest and the extensions that we know have been
        // modified.
        let mut extensions_to_unload = Vec::default();
        let mut extensions_to_load = Vec::default();
        {
            let mut old_keys = old_index.extensions.iter().peekable();
            let mut new_keys = new_index.extensions.iter().peekable();
            loop {
                match (old_keys.peek(), new_keys.peek()) {
                    (None, None) => break,
                    (None, Some(_)) => {
                        extensions_to_load.push(new_keys.next().unwrap().0.clone());
                    }
                    (Some(_), None) => {
                        extensions_to_unload.push(old_keys.next().unwrap().0.clone());
                    }
                    (Some((old_key, _)), Some((new_key, _))) => match old_key.cmp(new_key) {
                        Ordering::Equal => {
                            let (old_key, old_value) = old_keys.next().unwrap();
                            let (new_key, new_value) = new_keys.next().unwrap();
                            if old_value != new_value || self.modified_extensions.contains(old_key) {
                                extensions_to_unload.push(old_key.clone());
                                extensions_to_load.push(new_key.clone());
                            }
                        }
                        Ordering::Less => {
                            extensions_to_unload.push(old_keys.next().unwrap().0.clone());
                        }
                        Ordering::Greater => {
                            extensions_to_load.push(new_keys.next().unwrap().0.clone());
                        }
                    },
                }
            }
            self.modified_extensions.clear();
        }

        if extensions_to_load.is_empty() && extensions_to_unload.is_empty() {
            self.reload_complete_senders.clear();
            return Task::ready(());
        }

        let reload_count = extensions_to_unload
            .iter()
            .filter(|id| extensions_to_load.contains(id))
            .count();

        log::info!(
            "extensions updated. loading {}, reloading {}, unloading {}",
            extensions_to_load.len() - reload_count,
            reload_count,
            extensions_to_unload.len() - reload_count
        );

        let themes_to_remove = old_index
            .themes
            .iter()
            .filter_map(|(name, entry)| {
                if extensions_to_unload.contains(&entry.extension) {
                    Some(name.clone().into())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let icon_themes_to_remove = old_index
            .icon_themes
            .iter()
            .filter_map(|(name, entry)| {
                if extensions_to_unload.contains(&entry.extension) {
                    Some(name.clone().into())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let languages_to_remove = old_index
            .languages
            .iter()
            .filter_map(|(name, entry)| {
                if extensions_to_unload.contains(&entry.extension) {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        let mut grammars_to_remove = Vec::new();
        let mut server_removal_tasks = Vec::with_capacity(extensions_to_unload.len());
        for extension_id in &extensions_to_unload {
            let Some(extension) = old_index.extensions.get(extension_id) else {
                continue;
            };
            grammars_to_remove.extend(extension.manifest.grammars.keys().cloned());
            for (language_server_name, config) in &extension.manifest.language_servers {
                for language in config.languages() {
                    server_removal_tasks.push(self.proxy.remove_language_server(&language, language_server_name, cx));
                }
            }

            for adapter in extension.manifest.debug_adapters.keys() {
                self.proxy.unregister_debug_adapter(adapter.clone());
            }
            for locator in extension.manifest.debug_locators.keys() {
                self.proxy.unregister_debug_locator(locator.clone());
            }
        }

        self.wasm_extensions
            .retain(|(extension, _)| !extensions_to_unload.contains(&extension.id));
        self.proxy.remove_user_themes(themes_to_remove);
        self.proxy.remove_icon_themes(icon_themes_to_remove);
        self.proxy.remove_languages(&languages_to_remove, &grammars_to_remove);

        let mut grammars_to_add = Vec::new();
        let mut themes_to_add = Vec::new();
        let mut icon_themes_to_add = Vec::new();
        let mut snippets_to_add = Vec::new();
        for extension_id in &extensions_to_load {
            let Some(extension) = new_index.extensions.get(extension_id) else {
                continue;
            };

            grammars_to_add.extend(extension.manifest.grammars.keys().map(|grammar_name| {
                let mut grammar_path = self.installed_dir.clone();
                grammar_path.extend([extension_id.as_ref(), "grammars"]);
                grammar_path.push(grammar_name.as_ref());
                grammar_path.set_extension("wasm");
                (grammar_name.clone(), grammar_path)
            }));
            themes_to_add.extend(extension.manifest.themes.iter().map(|theme_path| {
                let mut path = self.installed_dir.clone();
                path.extend([Path::new(extension_id.as_ref()), theme_path.as_path()]);
                path
            }));
            icon_themes_to_add.extend(extension.manifest.icon_themes.iter().map(|icon_theme_path| {
                let mut path = self.installed_dir.clone();
                path.extend([Path::new(extension_id.as_ref()), icon_theme_path.as_path()]);

                let mut icons_root_path = self.installed_dir.clone();
                icons_root_path.extend([Path::new(extension_id.as_ref())]);

                (path, icons_root_path)
            }));
            snippets_to_add.extend(extension.manifest.snippets.iter().map(|snippets_path| {
                let mut path = self.installed_dir.clone();
                path.extend([Path::new(extension_id.as_ref()), snippets_path.as_path()]);
                path
            }));
        }

        self.proxy.register_grammars(grammars_to_add);
        let languages_to_add = new_index
            .languages
            .iter()
            .filter(|(_, entry)| extensions_to_load.contains(&entry.extension))
            .collect::<Vec<_>>();
        for (language_name, language) in languages_to_add {
            let mut language_path = self.installed_dir.clone();
            language_path.extend([Path::new(language.extension.as_ref()), language.path.as_path()]);
            self.proxy.register_language(
                language_name.clone(),
                language.grammar.clone(),
                language.matcher.clone(),
                language.hidden,
                Arc::new(move || {
                    let config = std::fs::read_to_string(language_path.join("config.toml"))?;
                    let config: LanguageConfig = ::toml::from_str(&config)?;
                    let queries = load_plugin_queries(&language_path);
                    let context_provider = std::fs::read_to_string(language_path.join("tasks.jsonc"))
                        .ok()
                        .or_else(|| std::fs::read_to_string(language_path.join("tasks.json")).ok())
                        .and_then(|contents| {
                            let definitions = serde_json_lenient::from_str(&contents).log_err()?;
                            Some(Arc::new(ContextProviderWithTasks::new(definitions)) as Arc<_>)
                        });

                    Ok(LoadedLanguage {
                        config,
                        queries,
                        context_provider,
                        toolchain_provider: None,
                        manifest_name: None,
                    })
                }),
            );
        }

        let fs = self.fs.clone();
        let wasm_host = self.wasm_host.clone();
        let root_dir = self.installed_dir.clone();
        let proxy = self.proxy.clone();
        let extension_entries = extensions_to_load
            .iter()
            .filter_map(|name| new_index.extensions.get(name).cloned())
            .collect::<Vec<_>>();
        self.extension_index = new_index;
        cx.notify();
        cx.emit(Event::ExtensionsUpdated);

        cx.spawn(async move |this, cx| {
            cx.background_spawn({
                let fs = fs.clone();
                async move {
                    let _ = join_all(server_removal_tasks).await;
                    for theme_path in themes_to_add {
                        proxy.load_user_theme(theme_path, fs.clone()).await.log_err();
                    }

                    for (icon_theme_path, icons_root_path) in icon_themes_to_add {
                        proxy
                            .load_icon_theme(icon_theme_path, icons_root_path, fs.clone())
                            .await
                            .log_err();
                    }

                    for snippets_path in &snippets_to_add {
                        match fs
                            .load(snippets_path)
                            .await
                            .with_context(|| format!("Loading snippets from {snippets_path:?}"))
                        {
                            Ok(snippets_contents) => {
                                proxy.register_snippet(snippets_path, &snippets_contents).log_err();
                            }
                            Err(e) => log::error!("Cannot load snippets: {e:#}"),
                        }
                    }
                }
            })
            .await;

            let mut wasm_extensions = Vec::new();
            for extension in extension_entries {
                if extension.manifest.lib.kind.is_none() {
                    continue;
                };

                let extension_path = root_dir.join(extension.manifest.id.as_ref());
                let wasm_extension = WasmExtension::load(&extension_path, &extension.manifest, wasm_host.clone(), cx)
                    .await
                    .with_context(|| format!("Loading extension from {extension_path:?}"));

                match wasm_extension {
                    Ok(wasm_extension) => wasm_extensions.push((extension.manifest.clone(), wasm_extension)),
                    Err(e) => {
                        log::error!("Failed to load extension: {}, {:#}", extension.manifest.id, e);
                        this.update(cx, |_, cx| {
                            cx.emit(Event::ExtensionFailedToLoad(extension.manifest.id.clone()))
                        })
                        .ok();
                    }
                }
            }

            this.update(cx, |this, cx| {
                this.reload_complete_senders.clear();

                for (manifest, wasm_extension) in &wasm_extensions {
                    let extension = Arc::new(wasm_extension.clone());

                    for (language_server_id, language_server_config) in &manifest.language_servers {
                        for language in language_server_config.languages() {
                            this.proxy.register_language_server(
                                extension.clone(),
                                language_server_id.clone(),
                                language.clone(),
                            );
                        }
                    }

                    for (debug_adapter, meta) in &manifest.debug_adapters {
                        let mut path = root_dir.clone();
                        path.push(Path::new(manifest.id.as_ref()));
                        if let Some(schema_path) = &meta.schema_path {
                            path.push(schema_path);
                        } else {
                            path.push("debug_adapter_schemas");
                            path.push(Path::new(debug_adapter.as_ref()).with_extension("json"));
                        }

                        this.proxy
                            .register_debug_adapter(extension.clone(), debug_adapter.clone(), &path);
                    }

                    for debug_adapter in manifest.debug_locators.keys() {
                        this.proxy
                            .register_debug_locator(extension.clone(), debug_adapter.clone());
                    }
                }

                this.wasm_extensions.extend(wasm_extensions);
                this.proxy.set_extensions_loaded();
                this.proxy.reload_current_theme(cx);
                this.proxy.reload_current_icon_theme(cx);

                if let Some(events) = ExtensionEvents::try_global(cx) {
                    events.update(cx, |this, cx| {
                        this.emit(extension::Event::ExtensionsInstalledChanged, cx)
                    });
                }
            })
            .ok();
        })
    }

    fn rebuild_extension_index(&self, cx: &mut Context<Self>) -> Task<ExtensionIndex> {
        let fs = self.fs.clone();
        let work_dir = self.wasm_host.work_dir.clone();
        let extensions_dir = self.installed_dir.clone();
        let index_path = self.index_path.clone();
        let proxy = self.proxy.clone();
        cx.background_spawn(async move {
            let start_time = Instant::now();
            let mut index = ExtensionIndex::default();

            fs.create_dir(&work_dir).await.log_err();
            fs.create_dir(&extensions_dir).await.log_err();
            log::info!("Rebuilding extension index at {:?}", index_path);
            log::info!("  from {:?}", extensions_dir);
            log::info!("  in {:?}", work_dir);

            let extension_paths = fs.read_dir(&extensions_dir).await;
            if let Ok(mut extension_paths) = extension_paths {
                while let Some(extension_dir) = extension_paths.next().await {
                    let Ok(extension_dir) = extension_dir else {
                        log::info!("Not ok: {:?}", extension_dir);
                        continue;
                    };

                    if extension_dir
                        .file_name()
                        .is_some_and(|file_name| file_name == ".DS_Store")
                    {
                        continue;
                    }

                    Self::add_extension_to_index(fs.clone(), extension_dir, &mut index, proxy.clone())
                        .await
                        .log_err();
                }
            }

            if let Ok(index_json) = serde_json::to_string_pretty(&index) {
                fs.save(&index_path, &index_json.as_str().into(), Default::default())
                    .await
                    .context("failed to save extension index")
                    .log_err();
            }

            log::info!("rebuilt extension index in {:?}", start_time.elapsed());
            index
        })
    }

    async fn add_extension_to_index(
        fs: Arc<dyn Fs>,
        extension_dir: PathBuf,
        index: &mut ExtensionIndex,
        proxy: Arc<ExtensionHostProxy>,
    ) -> Result<()> {
        let mut extension_manifest = ExtensionManifest::load(fs.clone(), &extension_dir).await?;
        let extension_id = extension_manifest.id.clone();

        if SUPPRESSED_EXTENSIONS.contains(&extension_id.as_ref()) {
            return Ok(());
        }

        // TODO: distinguish dev extensions more explicitly, by the absence
        // of a checksum file that we'll create when downloading normal extensions.
        let is_dev = fs
            .metadata(&extension_dir)
            .await?
            .with_context(|| format!("missing extension directory {extension_dir:?}"))?
            .is_symlink;

        log::info!("Found manifest: {:?} (dev: {})", extension_manifest, is_dev);

        let language_dir = extension_dir.join("languages");
        if let Ok(mut language_paths) = fs.read_dir(&language_dir).await {
            while let Some(language_path) = language_paths.next().await {
                let language_path =
                    language_path.with_context(|| format!("reading entries in language dir {language_dir:?}"))?;
                let Ok(relative_path) = language_path.strip_prefix(&extension_dir) else {
                    continue;
                };
                let Ok(Some(fs_metadata)) = fs.metadata(&language_path).await else {
                    continue;
                };
                if !fs_metadata.is_dir {
                    continue;
                }
                let language_config_path = language_path.join("config.toml");
                let config = fs
                    .load(&language_config_path)
                    .await
                    .with_context(|| format!("loading language config from {language_config_path:?}"))?;
                let config = ::toml::from_str::<LanguageConfig>(&config)?;

                let relative_path = relative_path.to_path_buf();
                if !extension_manifest.languages.contains(&relative_path) {
                    extension_manifest.languages.push(relative_path.clone());
                }

                index.languages.insert(
                    config.name.clone(),
                    ExtensionIndexLanguageEntry {
                        extension: extension_id.clone(),
                        path: relative_path,
                        matcher: config.matcher,
                        hidden: config.hidden,
                        grammar: config.grammar,
                    },
                );
            }
        }

        if let Ok(mut theme_paths) = fs.read_dir(&extension_dir.join("themes")).await {
            while let Some(theme_path) = theme_paths.next().await {
                let theme_path = theme_path?;
                let Ok(relative_path) = theme_path.strip_prefix(&extension_dir) else {
                    continue;
                };

                let Some(theme_families) = proxy.list_theme_names(theme_path.clone(), fs.clone()).await.log_err()
                else {
                    continue;
                };

                let relative_path = relative_path.to_path_buf();
                if !extension_manifest.themes.contains(&relative_path) {
                    extension_manifest.themes.push(relative_path.clone());
                }

                for theme_name in theme_families {
                    index.themes.insert(
                        theme_name.into(),
                        ExtensionIndexThemeEntry {
                            extension: extension_id.clone(),
                            path: relative_path.clone(),
                        },
                    );
                }
            }
        }

        if let Ok(mut icon_theme_paths) = fs.read_dir(&extension_dir.join("icon_themes")).await {
            while let Some(icon_theme_path) = icon_theme_paths.next().await {
                let icon_theme_path = icon_theme_path?;
                let Ok(relative_path) = icon_theme_path.strip_prefix(&extension_dir) else {
                    continue;
                };

                let Some(icon_theme_families) = proxy
                    .list_icon_theme_names(icon_theme_path.clone(), fs.clone())
                    .await
                    .log_err()
                else {
                    continue;
                };

                let relative_path = relative_path.to_path_buf();
                if !extension_manifest.icon_themes.contains(&relative_path) {
                    extension_manifest.icon_themes.push(relative_path.clone());
                }

                for icon_theme_name in icon_theme_families {
                    index.icon_themes.insert(
                        icon_theme_name.into(),
                        ExtensionIndexIconThemeEntry {
                            extension: extension_id.clone(),
                            path: relative_path.clone(),
                        },
                    );
                }
            }
        }

        let extension_wasm_path = extension_dir.join("extension.wasm");
        if fs.is_file(&extension_wasm_path).await {
            extension_manifest.lib.kind.get_or_insert(ExtensionLibraryKind::Rust);
        }

        index.extensions.insert(
            extension_id.clone(),
            ExtensionIndexEntry {
                dev: is_dev,
                manifest: Arc::new(extension_manifest),
            },
        );

        Ok(())
    }

    fn prepare_remote_extension(
        &mut self,
        extension_id: Arc<str>,
        is_dev: bool,
        tmp_dir: PathBuf,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        let src_dir = self.extensions_dir().join(extension_id.as_ref());
        let Some(loaded_extension) = self.extension_index.extensions.get(&extension_id).cloned() else {
            return Task::ready(Err(anyhow!("extension no longer installed")));
        };
        let fs = self.fs.clone();
        cx.background_spawn(async move {
            const EXTENSION_TOML: &str = "extension.toml";
            const EXTENSION_WASM: &str = "extension.wasm";
            const CONFIG_TOML: &str = "config.toml";

            if is_dev {
                let manifest_toml = toml::to_string(&loaded_extension.manifest)?;
                fs.save(
                    &tmp_dir.join(EXTENSION_TOML),
                    &Rope::from(manifest_toml),
                    language::LineEnding::Unix,
                )
                .await?;
            } else {
                fs.copy_file(
                    &src_dir.join(EXTENSION_TOML),
                    &tmp_dir.join(EXTENSION_TOML),
                    fs::CopyOptions::default(),
                )
                .await?
            }

            if fs.is_file(&src_dir.join(EXTENSION_WASM)).await {
                fs.copy_file(
                    &src_dir.join(EXTENSION_WASM),
                    &tmp_dir.join(EXTENSION_WASM),
                    fs::CopyOptions::default(),
                )
                .await?
            }

            for language_path in loaded_extension.manifest.languages.iter() {
                if fs.is_file(&src_dir.join(language_path).join(CONFIG_TOML)).await {
                    fs.create_dir(&tmp_dir.join(language_path)).await?;
                    fs.copy_file(
                        &src_dir.join(language_path).join(CONFIG_TOML),
                        &tmp_dir.join(language_path).join(CONFIG_TOML),
                        fs::CopyOptions::default(),
                    )
                    .await?
                }
            }

            for (adapter_name, meta) in loaded_extension.manifest.debug_adapters.iter() {
                let schema_path = &extension::build_debug_adapter_schema_path(adapter_name, meta);

                if fs.is_file(&src_dir.join(schema_path)).await {
                    if let Some(parent) = schema_path.parent() {
                        fs.create_dir(&tmp_dir.join(parent)).await?
                    }
                    fs.copy_file(
                        &src_dir.join(schema_path),
                        &tmp_dir.join(schema_path),
                        fs::CopyOptions::default(),
                    )
                    .await?
                }
            }

            Ok(())
        })
    }

    async fn sync_extensions_to_remotes(
        this: &WeakEntity<Self>,
        client: WeakEntity<RemoteClient>,
        cx: &mut AsyncApp,
    ) -> Result<()> {
        let extensions = this.update(cx, |this, _cx| {
            this.extension_index
                .extensions_to_sync_to_remote()
                .into_entries()
                .map(|(id, entry)| proto::Extension {
                    id: id.to_string(),
                    version: entry.manifest.version.to_string(),
                    dev: entry.dev,
                })
                .collect()
        })?;

        let response = client
            .update(cx, |client, _cx| {
                client.proto_client().request(proto::SyncExtensions { extensions })
            })?
            .await?;
        let path_style = client.read_with(cx, |client, _| client.path_style())?;

        for missing_extension in response.missing_extensions.into_iter() {
            let tmp_dir = tempfile::tempdir()?;
            this.update(cx, |this, cx| {
                this.prepare_remote_extension(
                    missing_extension.id.clone().into(),
                    missing_extension.dev,
                    tmp_dir.path().to_owned(),
                    cx,
                )
            })?
            .await?;
            let dest_dir = RemotePathBuf::new(
                path_style
                    .join(&response.tmp_dir, &missing_extension.id)
                    .with_context(|| {
                        format!(
                            "failed to construct destination path: {:?}, {:?}",
                            response.tmp_dir, missing_extension.id,
                        )
                    })?,
                path_style,
            );
            log::info!("Uploading extension {} to {:?}", missing_extension.clone().id, dest_dir);

            client
                .update(cx, |client, cx| {
                    client.upload_directory(tmp_dir.path().to_owned(), dest_dir.clone(), cx)
                })?
                .await?;

            log::info!("Finished uploading extension {}", missing_extension.clone().id);

            let result = client
                .update(cx, |client, _cx| {
                    client.proto_client().request(proto::InstallExtension {
                        tmp_dir: dest_dir.to_proto(),
                        extension: Some(missing_extension.clone()),
                    })
                })?
                .await;

            if let Err(e) = result {
                log::error!("Failed to install extension {}: {}", missing_extension.id, e);
            }
        }

        anyhow::Ok(())
    }

    pub async fn update_remote_clients(this: &WeakEntity<Self>, cx: &mut AsyncApp) -> Result<()> {
        let clients = this.update(cx, |this, _cx| {
            this.remote_clients.retain(|v| v.upgrade().is_some());
            this.remote_clients.clone()
        })?;

        for client in clients {
            Self::sync_extensions_to_remotes(this, client, cx).await.log_err();
        }

        anyhow::Ok(())
    }

    pub fn register_remote_client(&mut self, client: Entity<RemoteClient>, _cx: &mut Context<Self>) {
        self.remote_clients.push(client.downgrade());
        self.ssh_registered_tx.unbounded_send(()).ok();
    }
}

fn load_plugin_queries(root_path: &Path) -> LanguageQueries {
    let mut result = LanguageQueries::default();
    if let Some(entries) = std::fs::read_dir(root_path).log_err() {
        for entry in entries {
            let Some(entry) = entry.log_err() else {
                continue;
            };
            let path = entry.path();
            if let Some(remainder) = path.strip_prefix(root_path).ok().and_then(|p| p.to_str()) {
                if !remainder.ends_with(".scm") {
                    continue;
                }
                for (name, query) in QUERY_FILENAME_PREFIXES {
                    if remainder.starts_with(name) {
                        if let Some(contents) = std::fs::read_to_string(&path).log_err() {
                            match query(&mut result) {
                                None => *query(&mut result) = Some(contents.into()),
                                Some(r) => r.to_mut().push_str(contents.as_ref()),
                            }
                        }
                        break;
                    }
                }
            }
        }
    }
    result
}

/// cp -r from to, but also skipping hidden files (like .git)
/// as well as the target/ dir
async fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    create_dir_all(to)?;
    for entry in read_dir(from)? {
        let entry = entry?;
        let filetype = entry.file_type()?;
        if let Some(name) = entry.file_name().to_str()
            && (name.starts_with(".") || (name == "target" && filetype.is_dir()))
        {
            continue;
        }
        if filetype.is_dir() {
            Box::pin(copy_dir(entry.path().as_path(), &to.join(entry.file_name()))).await?;
        } else {
            copy(entry.path(), to.join(entry.file_name()))?;
        }
    }
    Ok(())
}
