mod components;
mod extension_suggest;

use app_actions::ExtensionCategoryFilter;
use client::{ExtensionMetadata, ExtensionProvides};
use collections::{BTreeMap, BTreeSet};
use editor::{Editor, EditorElement, EditorStyle};
use extension_host::{ExtensionManifest, ExtensionOperation, ExtensionStore};
use fuzzy::{StringMatchCandidate, match_strings};
use gpui::{
    Action, App, ClipboardItem, Context, Corner, DismissEvent, Entity, EventEmitter, Flatten, FocusHandle, Focusable,
    InteractiveElement, KeyContext, ParentElement, Point, Render, Styled, Task, TextStyle, UniformListScrollHandle,
    WeakEntity, Window, actions, point, uniform_list,
};
use menu;
use num_format::{Locale, ToFormattedString};
use project::DirectoryLister;
use schemars::JsonSchema;
use serde::Deserialize;
use settings::{Settings, SettingsContent};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;
use std::{ops::Range, sync::Arc};
use strum::IntoEnumIterator as _;
use theme::ThemeSettings;
use ui::{
    Banner, Chip, ContextMenu, Divider, Modal, ModalHeader, PopoverMenu, ScrollableHandle, Section, Switch, Tooltip,
    WithScrollbar, prelude::*,
};
use vim_mode_setting::VimModeSetting;
use workspace::ModalView;
use workspace::{
    Workspace,
    item::{Item, ItemEvent},
};

use crate::components::ExtensionCard;

actions!(
    monyacode,
    [
        /// Installs an extension from a local directory.
        InstallExtensionFromFolder,
        /// Installs an extension from a URL to a git repository.
        InstallExtensionFromUrl,
    ]
);

/// Install an extension from a local path
#[derive(PartialEq, Clone, Debug, Deserialize, JsonSchema, Action)]
#[action(namespace = monyacode)]
#[serde(deny_unknown_fields)]
pub struct InstallExtensionFromLocalPath {
    /// A local path to an extension
    pub path: String,
}

/// Install an extension from a git repository
#[derive(PartialEq, Clone, Debug, Deserialize, JsonSchema, Action)]
#[action(namespace = monyacode)]
#[serde(deny_unknown_fields)]
pub struct InstallExtensionFromGit {
    /// A git https url
    pub url: String,
}

pub fn init(cx: &mut App) {
    cx.observe_new(move |workspace: &mut Workspace, window, cx| {
        let Some(window) = window else {
            return;
        };
        workspace
            .register_action(move |workspace, action: &app_actions::Extensions, window, cx| {
                let provides_filter = action.category_filter.map(|category| match category {
                    ExtensionCategoryFilter::Themes => ExtensionProvides::Themes,
                    ExtensionCategoryFilter::IconThemes => ExtensionProvides::IconThemes,
                    ExtensionCategoryFilter::Languages => ExtensionProvides::Languages,
                    ExtensionCategoryFilter::Grammars => ExtensionProvides::Grammars,
                    ExtensionCategoryFilter::LanguageServers => ExtensionProvides::LanguageServers,
                    ExtensionCategoryFilter::IndexedDocsProviders => ExtensionProvides::IndexedDocsProviders,
                    ExtensionCategoryFilter::Snippets => ExtensionProvides::Snippets,
                    ExtensionCategoryFilter::DebugAdapters => ExtensionProvides::DebugAdapters,
                });

                let existing = workspace
                    .active_pane()
                    .read(cx)
                    .items()
                    .find_map(|item| item.downcast::<ExtensionsPage>());

                if let Some(existing) = existing {
                    existing.update(cx, |extensions_page, cx| {
                        if provides_filter.is_some() {
                            extensions_page.change_provides_filter(provides_filter, cx);
                        }
                        if let Some(id) = action.id.as_ref() {
                            extensions_page.focus_extension(id, window, cx);
                        }
                    });

                    workspace.activate_item(&existing, true, true, window, cx);
                } else {
                    let extensions_page =
                        ExtensionsPage::new(workspace, provides_filter, action.id.as_deref(), window, cx);
                    workspace.add_item_to_active_pane(Box::new(extensions_page), None, true, window, cx)
                }
            })
            .register_action(move |workspace, _: &InstallExtensionFromFolder, window, cx| {
                let prompt = workspace.prompt_for_open_path(
                    gpui::PathPromptOptions {
                        files: false,
                        directories: true,
                        multiple: false,
                        prompt: None,
                    },
                    DirectoryLister::Local(workspace.project().clone(), workspace.app_state().fs.clone()),
                    window,
                    cx,
                );
                let workspace_handle = cx.entity().downgrade();
                window
                    .spawn(cx, async move |cx| {
                        let path = match Flatten::flatten(prompt.await.map_err(|e| e.into())) {
                            Ok(Some(mut paths)) => match paths.pop() {
                                Some(path) => path,
                                None => return,
                            },
                            Ok(None) => return,
                            Err(err) => {
                                workspace_handle
                                    .update(cx, |workspace, cx| {
                                        workspace.show_portal_error(err.to_string(), cx);
                                    })
                                    .ok();
                                return;
                            }
                        };
                        let Some(path) = path.into_os_string().into_string().ok() else {
                            return;
                        };
                        cx.update(|window, cx| {
                            window.dispatch_action(Box::new(InstallExtensionFromLocalPath { path }), cx);
                        })
                        .ok();
                    })
                    .detach();
            })
            .register_action(move |workspace, _: &InstallExtensionFromUrl, window, cx| {
                workspace.toggle_modal(window, cx, |window, cx| GitCloneModal::show(window, cx));
            })
            .register_action(move |workspace, action: &InstallExtensionFromGit, window, cx| {
                let url = action.url.clone();
                workspace
                    .with_local_workspace(window, cx, move |workspace, window, cx| {
                        let file_stem = match url.split('/').next_back() {
                            Some(tail) => {
                                if let Some(stripped) = tail.strip_suffix(".git") {
                                    stripped
                                } else {
                                    tail
                                }
                            }
                            None => {
                                workspace.show_error_with_link(
                                    "Invalid Git URL format: Could not split on /",
                                    "Learn more",
                                    "monyacode://docs/extensions/installing-extensions",
                                    cx,
                                );
                                return;
                            }
                        };
                        let temp_dir = match tempfile::TempDir::with_suffix(file_stem) {
                            Ok(temp_dir) => temp_dir,
                            Err(err) => {
                                workspace.show_error_with_link(
                                    SharedString::from(format!("Failed to create temporary directory:\n{err:#}")),
                                    "Learn more".into(),
                                    "monyacode://docs/extensions/installing-extensions".into(),
                                    cx,
                                );
                                return;
                            }
                        };

                        let url = url.clone();
                        let destination_dir = temp_dir.keep();
                        let repo_name = url
                            .split('/')
                            .next_back()
                            .map(|name| name.strip_suffix(".git").unwrap_or(name))
                            .unwrap_or("repository")
                            .to_owned();
                        let mut repo_dir = destination_dir.clone();
                        repo_dir.push(&repo_name);
                        let fs = workspace.app_state().fs.clone();
                        cx.spawn_in(window, async move |workspace, cx| {
                            let repo_url = url.as_str();
                            match fs.git_clone(&repo_url, destination_dir.as_path()).await {
                                Ok(_) => {}
                                Err(err) => {
                                    return workspace.update(cx, |workspace, cx| {
                                        workspace.show_error_with_link(
                                            SharedString::from(format!("Failed to clone Git repository:\n{err:#}")),
                                            "Learn more".into(),
                                            "monyacode://docs/extensions/installing-extensions".into(),
                                            cx,
                                        );
                                    });
                                }
                            }
                            let path = match repo_dir.to_str() {
                                Some(path) => path.to_string(),
                                None => {
                                    return workspace.update(cx, |workspace, cx| {
                                        workspace.show_error_with_link(
                                            "Failed to clone Git repository: No workdir found",
                                            "Learn more",
                                            "monyacode://docs/extensions/installing-extensions",
                                            cx,
                                        );
                                    });
                                }
                            };
                            cx.update(|window, cx| {
                                window.dispatch_action(Box::new(InstallExtensionFromLocalPath { path }), cx)
                            })
                        })
                        .detach();
                    })
                    .detach();
            })
            .register_action(move |_workspace, action: &InstallExtensionFromLocalPath, window, cx| {
                let store = ExtensionStore::global(cx);
                let extension_path = PathBuf::from(&action.path);
                let workspace_handle = cx.entity().downgrade();
                window
                    .spawn(cx, async move |cx| {
                        let install_task = store
                            .update(cx, |store, cx| store.install_dev_extension(extension_path, cx))
                            .ok()?;

                        match install_task.await {
                            Ok(_) => {}
                            Err(err) => {
                                log::error!("Failed to install extension: {:?}", err);
                                workspace_handle
                                    .update(cx, |workspace, cx| {
                                        workspace.show_error_with_link(
                                            SharedString::from(format!("Failed to install extension:\n{err:#}")),
                                            "Learn more".into(),
                                            "monyacode://docs/extensions/installing-extensions".into(),
                                            cx,
                                        );
                                    })
                                    .ok();
                            }
                        }

                        Some(())
                    })
                    .detach();
            });

        cx.subscribe_in(workspace.project(), window, |_, _, event, window, cx| {
            if let project::Event::LanguageNotFound(buffer) = event {
                extension_suggest::suggest(buffer.clone(), window, cx);
            }
        })
        .detach();
    })
    .detach();
}

fn extension_provides_label(provides: ExtensionProvides) -> &'static str {
    match provides {
        ExtensionProvides::Themes => "Themes",
        ExtensionProvides::IconThemes => "Icon Themes",
        ExtensionProvides::Languages => "Languages",
        ExtensionProvides::Grammars => "Grammars",
        ExtensionProvides::LanguageServers => "Language Servers",
        ExtensionProvides::IndexedDocsProviders => "Indexed Docs Providers",
        ExtensionProvides::Snippets => "Snippets",
        ExtensionProvides::DebugAdapters => "Debug Adapters",
    }
}

#[derive(Clone)]
pub enum ExtensionStatus {
    NotInstalled,
    Installing,
    Upgrading,
    Installed(Arc<str>),
    Removing,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
enum Feature {
    ExtensionBasedpyright,
    ExtensionRuff,
    ExtensionTailwind,
    ExtensionTy,
    Git,
    LanguageBash,
    LanguageC,
    LanguageCpp,
    LanguageGo,
    LanguagePython,
    LanguageReact,
    LanguageRust,
    LanguageTypescript,
    OpenIn,
    Vim,
}

fn keywords_by_feature() -> &'static BTreeMap<Feature, Vec<&'static str>> {
    static KEYWORDS_BY_FEATURE: OnceLock<BTreeMap<Feature, Vec<&'static str>>> = OnceLock::new();
    KEYWORDS_BY_FEATURE.get_or_init(|| {
        BTreeMap::from_iter([
            (Feature::ExtensionBasedpyright, vec!["basedpyright", "pyright"]),
            (Feature::ExtensionRuff, vec!["ruff"]),
            (Feature::ExtensionTailwind, vec!["tail", "tailwind"]),
            (Feature::ExtensionTy, vec!["ty"]),
            (Feature::Git, vec!["git"]),
            (Feature::LanguageBash, vec!["sh", "bash"]),
            (Feature::LanguageC, vec!["c", "clang"]),
            (Feature::LanguageCpp, vec!["c++", "cpp", "clang"]),
            (Feature::LanguageGo, vec!["go", "golang"]),
            (Feature::LanguagePython, vec!["python", "py"]),
            (Feature::LanguageReact, vec!["react"]),
            (Feature::LanguageRust, vec!["rust", "rs"]),
            (Feature::LanguageTypescript, vec!["type", "typescript", "ts"]),
            (
                Feature::OpenIn,
                vec![
                    "github",
                    "gitlab",
                    "bitbucket",
                    "codeberg",
                    "sourcehut",
                    "permalink",
                    "link",
                    "open in",
                ],
            ),
            (Feature::Vim, vec!["vim"]),
        ])
    })
}

struct ExtensionCardButtons {
    install_or_uninstall: Button,
    upgrade: Option<Button>,
    configure: Option<Button>,
}

pub struct ExtensionsPage {
    workspace: WeakEntity<Workspace>,
    list: UniformListScrollHandle,
    is_fetching_extensions: bool,
    fetch_failed: bool,
    remote_extension_entries: Vec<ExtensionMetadata>,
    extension_entries: Vec<Arc<ExtensionManifest>>,
    filtered_remote_extension_indices: Vec<usize>,
    query_editor: Entity<Editor>,
    query_contains_error: bool,
    provides_filter: Option<ExtensionProvides>,
    _subscriptions: [gpui::Subscription; 2],
    extension_fetch_task: Option<Task<()>>,
    upsells: BTreeSet<Feature>,
}

impl ExtensionsPage {
    pub fn new(
        workspace: &Workspace,
        provides_filter: Option<ExtensionProvides>,
        focus_extension_id: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) -> Entity<Self> {
        cx.new(|cx| {
            let store = ExtensionStore::global(cx);
            let workspace_handle = workspace.weak_handle();
            let subscriptions = [
                cx.observe(&store, |_: &mut Self, _, cx| cx.notify()),
                cx.subscribe_in(&store, window, move |this, _, event, window, cx| match event {
                    extension_host::Event::ExtensionsUpdated => this.fetch_extensions_debounced(None, cx),
                    extension_host::Event::ExtensionInstalled(extension_id) => {
                        this.on_extension_installed(workspace_handle.clone(), extension_id, window, cx)
                    }
                    _ => {}
                }),
            ];

            let query_editor = cx.new(|cx| {
                let mut input = Editor::single_line(window, cx);
                input.set_placeholder_text("Search extensions...", window, cx);
                if let Some(id) = focus_extension_id {
                    input.set_text(format!("id:{id}"), window, cx);
                }
                input
            });
            cx.subscribe(&query_editor, Self::on_query_change).detach();

            let scroll_handle = UniformListScrollHandle::new();

            let mut this = Self {
                workspace: workspace.weak_handle(),
                list: scroll_handle,
                is_fetching_extensions: false,
                fetch_failed: false,
                extension_entries: Vec::new(),
                filtered_remote_extension_indices: Vec::new(),
                remote_extension_entries: Vec::new(),
                query_contains_error: false,
                provides_filter,
                extension_fetch_task: None,
                _subscriptions: subscriptions,
                query_editor,
                upsells: BTreeSet::default(),
            };
            this.fetch_extensions(
                this.search_query(cx),
                Some(BTreeSet::from_iter(this.provides_filter)),
                None,
                cx,
            );
            this
        })
    }

    fn fetch_extensions(
        &mut self,
        search: Option<String>,
        provides_filter: Option<BTreeSet<ExtensionProvides>>,
        on_complete: Option<Box<dyn FnOnce(&mut Self, &mut Context<Self>) + Send>>,
        cx: &mut Context<Self>,
    ) {
        self.is_fetching_extensions = true;
        self.fetch_failed = false;
        cx.notify();

        let extension_store = ExtensionStore::global(cx);
        let provides_filter = provides_filter.unwrap_or(BTreeSet::default());

        let extensions = extension_store
            .read(cx)
            .installed_extensions()
            .values()
            .map(|ext| &ext.manifest)
            .filter(|ext| {
                provides_filter.is_empty() || extension_provides(ext).intersection(&provides_filter).count() > 0
            })
            .cloned()
            .collect::<Vec<Arc<ExtensionManifest>>>();

        log::info!("extensions: {:?}", extensions);

        cx.spawn(async move |this, cx| {
            let extensions = if let Some(search) = search {
                let match_candidates = extensions
                    .iter()
                    .enumerate()
                    .map(|(ix, manifest)| StringMatchCandidate::new(ix, &manifest.name))
                    .collect::<Vec<_>>();

                let matches = match_strings(
                    &match_candidates,
                    &search,
                    false,
                    true,
                    match_candidates.len(),
                    &Default::default(),
                    cx.background_executor().clone(),
                )
                .await;
                matches
                    .into_iter()
                    .map(|mat| extensions[mat.candidate_id].clone())
                    .collect()
            } else {
                extensions
            };

            this.update(cx, |this, cx| {
                cx.notify();
                this.extension_entries = extensions;
                this.is_fetching_extensions = false;
                this.fetch_failed = false;
                this.filter_extension_entries(cx);
                if let Some(callback) = on_complete {
                    callback(this, cx);
                }
            })
        })
        .detach_and_log_err(cx);
    }

    fn fetch_extensions_debounced(
        &mut self,
        on_complete: Option<Box<dyn FnOnce(&mut Self, &mut Context<Self>) + Send>>,
        cx: &mut Context<ExtensionsPage>,
    ) {
        self.extension_fetch_task = Some(cx.spawn(async move |this, cx| {
            let search = this.update(cx, |this, cx| this.search_query(cx)).ok().flatten();

            // Only debounce the fetching of extensions if we have a search
            // query.
            //
            // If the search was just cleared then we can just reload the list
            // of extensions without a debounce, which allows us to avoid seeing
            // an intermittent flash of a "no extensions" state.
            if search.is_some() {
                cx.background_executor().timer(Duration::from_millis(250)).await;
            };

            this.update(cx, |this, cx| {
                this.fetch_extensions(search, Some(BTreeSet::from_iter(this.provides_filter)), on_complete, cx);
            })
            .ok();
        }));
    }

    fn on_extension_installed(
        &mut self,
        workspace: WeakEntity<Workspace>,
        extension_id: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let extension_store = ExtensionStore::global(cx).read(cx);
        let themes = extension_store
            .extension_themes(extension_id)
            .map(|name| name.to_string())
            .collect::<Vec<_>>();
        if !themes.is_empty() {
            workspace
                .update(cx, |_workspace, cx| {
                    window.dispatch_action(
                        app_actions::theme_selector::Toggle {
                            themes_filter: Some(themes),
                        }
                        .boxed_clone(),
                        cx,
                    );
                })
                .ok();
            return;
        }

        let icon_themes = extension_store
            .extension_icon_themes(extension_id)
            .map(|name| name.to_string())
            .collect::<Vec<_>>();
        if !icon_themes.is_empty() {
            workspace
                .update(cx, |_workspace, cx| {
                    window.dispatch_action(
                        app_actions::icon_theme_selector::Toggle {
                            themes_filter: Some(icon_themes),
                        }
                        .boxed_clone(),
                        cx,
                    );
                })
                .ok();
        }
    }

    fn extension_status(extension_id: &str, cx: &mut Context<Self>) -> ExtensionStatus {
        let extension_store = ExtensionStore::global(cx).read(cx);

        match extension_store.outstanding_operations().get(extension_id) {
            Some(ExtensionOperation::Install) => ExtensionStatus::Installing,
            Some(ExtensionOperation::Remove) => ExtensionStatus::Removing,
            Some(ExtensionOperation::Upgrade) => ExtensionStatus::Upgrading,
            None => match extension_store.installed_extensions().get(extension_id) {
                Some(extension) => ExtensionStatus::Installed(extension.manifest.version.clone()),
                None => ExtensionStatus::NotInstalled,
            },
        }
    }

    fn filter_extension_entries(&mut self, cx: &mut Context<Self>) {
        self.filtered_remote_extension_indices.clear();
        self.filtered_remote_extension_indices
            .extend(self.remote_extension_entries.iter().enumerate().map(|(ix, _)| ix));
        cx.notify();
    }

    fn render_extensions(&mut self, range: Range<usize>, _: &mut Window, cx: &mut Context<Self>) -> Vec<ExtensionCard> {
        let extension_entries_len = self.extension_entries.len();
        range
            .map(|ix| {
                if ix < extension_entries_len {
                    let extension = &self.extension_entries[ix];
                    self.render_extension(extension, cx)
                } else {
                    let extension_ix = self.filtered_remote_extension_indices[ix - extension_entries_len];
                    let extension = &self.remote_extension_entries[extension_ix];
                    self.render_remote_extension(extension, cx)
                }
            })
            .collect()
    }

    fn render_extension(&self, extension: &ExtensionManifest, cx: &mut Context<Self>) -> ExtensionCard {
        let status = Self::extension_status(&extension.id, cx);

        let repository_url = extension.repository.clone();

        ExtensionCard::new()
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_end()
                            .child(Headline::new(extension.name.clone()).size(HeadlineSize::Medium))
                            .child(Headline::new(format!("v{}", extension.version)).size(HeadlineSize::XSmall)),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .justify_between()
                            .child(
                                Button::new(SharedString::from(format!("rebuild-{}", extension.id)), "Rebuild")
                                    .color(Color::Accent)
                                    .disabled(matches!(status, ExtensionStatus::Upgrading))
                                    .on_click({
                                        let extension_id = extension.id.clone();
                                        move |_, _, cx| {
                                            ExtensionStore::global(cx).update(cx, |store, cx| {
                                                store.rebuild_dev_extension(extension_id.clone(), cx)
                                            });
                                        }
                                    }),
                            )
                            .child(
                                Button::new(SharedString::from(extension.id.clone()), "Uninstall")
                                    .color(Color::Accent)
                                    .disabled(!matches!(status, ExtensionStatus::Installed(..)))
                                    .on_click({
                                        let extension_id = extension.id.clone();
                                        move |_, _, cx| {
                                            ExtensionStore::global(cx)
                                                .update(cx, |store, cx| {
                                                    store.uninstall_extension(extension_id.clone(), cx)
                                                })
                                                .detach_and_log_err(cx);
                                        }
                                    }),
                            ),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .justify_between()
                    .child(
                        Label::new(format!(
                            "{}: {}",
                            if extension.authors.len() > 1 {
                                "Authors"
                            } else {
                                "Author"
                            },
                            extension.authors.join(", ")
                        ))
                        .size(LabelSize::Small)
                        .color(Color::Muted)
                        .truncate(),
                    )
                    .child(Label::new("<>").size(LabelSize::Small)),
            )
            .child(
                h_flex()
                    .gap_2()
                    .justify_between()
                    .children(extension.description.as_ref().map(|description| {
                        Label::new(description.clone())
                            .size(LabelSize::Small)
                            .color(Color::Default)
                            .truncate()
                    }))
                    .children(repository_url.map(|repository_url| {
                        IconButton::new(
                            SharedString::from(format!("repository-{}", extension.id)),
                            IconName::Forge,
                        )
                        .icon_color(Color::Accent)
                        .icon_size(IconSize::Small)
                        .on_click(cx.listener({
                            let repository_url = repository_url.clone();
                            move |_, _, _, cx| {
                                cx.open_url(&repository_url);
                            }
                        }))
                        .tooltip(Tooltip::text(repository_url))
                    })),
            )
    }

    fn render_remote_extension(&self, extension: &ExtensionMetadata, cx: &mut Context<Self>) -> ExtensionCard {
        let this = cx.weak_entity();
        let status = Self::extension_status(&extension.id, cx);

        let extension_id = extension.id.clone();
        let buttons = self.buttons_for_entry(extension, &status, cx);
        let version = extension.manifest.version.clone();
        let repository_url = extension.manifest.repository.clone();
        let authors = extension.manifest.authors.clone();

        let installed_version = match status {
            ExtensionStatus::Installed(installed_version) => Some(installed_version),
            _ => None,
        };

        ExtensionCard::new()
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Headline::new(extension.manifest.name.clone()).size(HeadlineSize::Small))
                            .child(Headline::new(format!("v{version}")).size(HeadlineSize::XSmall))
                            .children(
                                installed_version
                                    .filter(|installed_version| *installed_version != version)
                                    .map(|installed_version| {
                                        Headline::new(format!("(v{installed_version} installed)",))
                                            .size(HeadlineSize::XSmall)
                                    }),
                            )
                            .map(|parent| {
                                if extension.manifest.provides.is_empty() {
                                    return parent;
                                }

                                parent.child(
                                    h_flex().gap_1().children(
                                        extension
                                            .manifest
                                            .provides
                                            .iter()
                                            .filter_map(|provides| {
                                                match provides {
                                                    ExtensionProvides::IndexedDocsProviders => {
                                                        return None;
                                                    }
                                                    _ => {}
                                                }

                                                Some(Chip::new(extension_provides_label(*provides)))
                                            })
                                            .collect::<Vec<_>>(),
                                    ),
                                )
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .children(buttons.upgrade)
                            .children(buttons.configure)
                            .child(buttons.install_or_uninstall),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .justify_between()
                    .children(extension.manifest.description.as_ref().map(|description| {
                        Label::new(description.clone())
                            .size(LabelSize::Small)
                            .color(Color::Default)
                            .truncate()
                    }))
                    .child(
                        Label::new(format!(
                            "Downloads: {}",
                            extension.download_count.to_formatted_string(&Locale::en)
                        ))
                        .size(LabelSize::Small),
                    ),
            )
            .child(
                h_flex()
                    .justify_between()
                    .child(
                        h_flex()
                            .gap_1()
                            .child(Icon::new(IconName::Person).size(IconSize::XSmall).color(Color::Muted))
                            .child(
                                Label::new(extension.manifest.authors.join(", "))
                                    .size(LabelSize::Small)
                                    .color(Color::Muted)
                                    .truncate(),
                            ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .child({
                                let repo_url_for_tooltip = repository_url.clone();

                                IconButton::new(
                                    SharedString::from(format!("repository-{}", extension.id)),
                                    IconName::Forge,
                                )
                                .icon_size(IconSize::Small)
                                .tooltip(move |_, cx| {
                                    Tooltip::with_meta(
                                        "Visit Extension Repository",
                                        None,
                                        repo_url_for_tooltip.clone(),
                                        cx,
                                    )
                                })
                                .on_click(cx.listener(move |_, _, _, cx| {
                                    cx.open_url(&repository_url);
                                }))
                            })
                            .child(
                                PopoverMenu::new(SharedString::from(format!("more-{}", extension.id)))
                                    .trigger(
                                        IconButton::new(
                                            SharedString::from(format!("more-{}", extension.id)),
                                            IconName::Ellipsis,
                                        )
                                        .icon_size(IconSize::Small),
                                    )
                                    .anchor(Corner::TopRight)
                                    .offset(Point { x: px(0.0), y: px(2.0) })
                                    .menu(move |window, cx| {
                                        this.upgrade().map(|this| {
                                            Self::render_remote_extension_context_menu(
                                                &this,
                                                extension_id.clone(),
                                                authors.clone(),
                                                window,
                                                cx,
                                            )
                                        })
                                    }),
                            ),
                    ),
            )
    }

    fn render_remote_extension_context_menu(
        _this: &Entity<Self>,
        extension_id: Arc<str>,
        authors: Vec<String>,
        window: &mut Window,
        cx: &mut App,
    ) -> Entity<ContextMenu> {
        ContextMenu::build(window, cx, |context_menu, _window, _| {
            context_menu
                .entry("Copy Extension ID", None, {
                    let extension_id = extension_id.clone();
                    move |_, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(extension_id.to_string()));
                    }
                })
                .entry("Copy Author Info", None, {
                    let authors = authors.clone();
                    move |_, cx| {
                        cx.write_to_clipboard(ClipboardItem::new_string(authors.join(", ")));
                    }
                })
        })
    }

    fn buttons_for_entry(
        &self,
        extension: &ExtensionMetadata,
        status: &ExtensionStatus,
        _cx: &mut Context<Self>,
    ) -> ExtensionCardButtons {
        let is_configurable = false;

        match status.clone() {
            ExtensionStatus::NotInstalled => ExtensionCardButtons {
                install_or_uninstall: Button::new(SharedString::from(extension.id.clone()), "Install")
                    .style(ButtonStyle::Tinted(ui::TintColor::Accent))
                    .icon(IconName::Download)
                    .icon_size(IconSize::Small)
                    .icon_color(Color::Muted)
                    .icon_position(IconPosition::Start)
                    .disabled(true),
                configure: None,
                upgrade: None,
            },
            ExtensionStatus::Installing => ExtensionCardButtons {
                install_or_uninstall: Button::new(SharedString::from(extension.id.clone()), "Install")
                    .style(ButtonStyle::Tinted(ui::TintColor::Accent))
                    .icon(IconName::Download)
                    .icon_size(IconSize::Small)
                    .icon_color(Color::Muted)
                    .icon_position(IconPosition::Start)
                    .disabled(true),
                configure: None,
                upgrade: None,
            },
            ExtensionStatus::Upgrading => ExtensionCardButtons {
                install_or_uninstall: Button::new(SharedString::from(extension.id.clone()), "Uninstall")
                    .style(ButtonStyle::OutlinedGhost)
                    .disabled(true),
                configure: is_configurable.then(|| {
                    Button::new(SharedString::from(format!("configure-{}", extension.id)), "Configure").disabled(true)
                }),
                upgrade: Some(Button::new(SharedString::from(extension.id.clone()), "Upgrade").disabled(true)),
            },
            ExtensionStatus::Installed(_installed_version) => ExtensionCardButtons {
                install_or_uninstall: Button::new(SharedString::from(extension.id.clone()), "Uninstall")
                    .style(ButtonStyle::OutlinedGhost)
                    .disabled(true),
                configure: is_configurable.then(|| {
                    Button::new(SharedString::from(format!("configure-{}", extension.id)), "Configure")
                        .style(ButtonStyle::OutlinedGhost)
                        .on_click({
                            let extension_id = extension.id.clone();
                            move |_, _, cx| {
                                if let Some(manifest) = ExtensionStore::global(cx)
                                    .read(cx)
                                    .extension_manifest_for_id(&extension_id)
                                    .cloned()
                                    && let Some(events) = extension::ExtensionEvents::try_global(cx)
                                {
                                    events.update(cx, |this, cx| {
                                        this.emit(extension::Event::ConfigureExtensionRequested(manifest), cx)
                                    });
                                }
                            }
                        })
                }),
                upgrade: None,
            },
            ExtensionStatus::Removing => ExtensionCardButtons {
                install_or_uninstall: Button::new(SharedString::from(extension.id.clone()), "Uninstall")
                    .style(ButtonStyle::OutlinedGhost)
                    .disabled(true),
                configure: is_configurable.then(|| {
                    Button::new(SharedString::from(format!("configure-{}", extension.id)), "Configure").disabled(true)
                }),
                upgrade: None,
            },
        }
    }

    fn render_search(&self, cx: &mut Context<Self>) -> Div {
        let mut key_context = KeyContext::new_with_defaults();
        key_context.add("BufferSearchBar");

        let editor_border = if self.query_contains_error {
            Color::Error.color(cx)
        } else {
            cx.theme().colors().border
        };

        h_flex()
            .key_context(key_context)
            .h_8()
            .min_w(rems(24.))
            .flex_1()
            .pl_1p5()
            .pr_2()
            .gap_2()
            .border_1()
            .border_color(editor_border)
            .rounded_md()
            .child(Icon::new(IconName::MagnifyingGlass).color(Color::Muted))
            .child(self.render_text_input(&self.query_editor, cx))
    }

    fn render_text_input(&self, editor: &Entity<Editor>, cx: &mut Context<Self>) -> impl IntoElement {
        let settings = ThemeSettings::get_global(cx);
        let text_style = TextStyle {
            color: if editor.read(cx).read_only(cx) {
                cx.theme().colors().text_disabled
            } else {
                cx.theme().colors().text
            },
            font_family: settings.ui_font.family.clone(),
            font_features: settings.ui_font.features.clone(),
            font_fallbacks: settings.ui_font.fallbacks.clone(),
            font_size: rems(0.875).into(),
            font_weight: settings.ui_font.weight,
            line_height: relative(1.3),
            ..Default::default()
        };

        EditorElement::new(
            editor,
            EditorStyle {
                background: cx.theme().colors().editor_background,
                local_player: cx.theme().players().local(),
                text: text_style,
                ..Default::default()
            },
        )
    }

    fn on_query_change(&mut self, _: Entity<Editor>, event: &editor::EditorEvent, cx: &mut Context<Self>) {
        if let editor::EditorEvent::Edited { .. } = event {
            self.query_contains_error = false;
            self.refresh_search(cx);
        }
    }

    fn refresh_search(&mut self, cx: &mut Context<Self>) {
        self.fetch_extensions_debounced(
            Some(Box::new(|this, cx| {
                this.scroll_to_top(cx);
            })),
            cx,
        );
        self.refresh_feature_upsells(cx);
    }

    fn scroll_to_top(&mut self, cx: &mut Context<Self>) {
        self.list.set_offset(point(px(0.), px(0.)));
        cx.notify();
    }
    pub fn focus_extension(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.query_editor
            .update(cx, |editor, cx| editor.set_text(format!("id:{id}"), window, cx));
        self.refresh_search(cx);
    }

    pub fn change_provides_filter(&mut self, provides_filter: Option<ExtensionProvides>, cx: &mut Context<Self>) {
        self.provides_filter = provides_filter;
        self.refresh_search(cx);
    }

    pub fn search_query(&self, cx: &mut App) -> Option<String> {
        let search = self.query_editor.read(cx).text(cx);
        if search.trim().is_empty() { None } else { Some(search) }
    }

    fn render_empty_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let has_search = self.search_query(cx).is_some();

        let message = if self.is_fetching_extensions {
            "Loading extensions…"
        } else if self.fetch_failed {
            "Failed to load extensions. Please check your connection and try again."
        } else if has_search {
            "No installed extensions that match your search."
        } else {
            "No installed extensions."
        };

        h_flex()
            .py_4()
            .gap_1p5()
            .when(self.fetch_failed, |this| {
                this.child(Icon::new(IconName::Warning).size(IconSize::Small).color(Color::Warning))
            })
            .child(Label::new(message))
    }

    fn update_settings(
        &mut self,
        selection: &ToggleState,

        cx: &mut Context<Self>,
        callback: impl 'static + Send + Fn(&mut SettingsContent, bool),
    ) {
        if let Some(workspace) = self.workspace.upgrade() {
            let fs = workspace.read(cx).app_state().fs.clone();
            let selection = *selection;
            settings::update_settings_file(fs, cx, move |settings, _| {
                let value = match selection {
                    ToggleState::Unselected => false,
                    ToggleState::Selected => true,
                    _ => return,
                };

                callback(settings, value)
            });
        }
    }

    fn refresh_feature_upsells(&mut self, cx: &mut Context<Self>) {
        let Some(search) = self.search_query(cx) else {
            self.upsells.clear();
            return;
        };

        if let Some(id) = search.strip_prefix("id:") {
            self.upsells.clear();

            let upsell = match id.to_lowercase().as_str() {
                "ruff" => Some(Feature::ExtensionRuff),
                "basedpyright" => Some(Feature::ExtensionBasedpyright),
                "ty" => Some(Feature::ExtensionTy),
                _ => None,
            };

            if let Some(upsell) = upsell {
                self.upsells.insert(upsell);
            }

            return;
        }

        let search = search.to_lowercase();
        let search_terms = search.split_whitespace().map(|term| term.trim()).collect::<Vec<_>>();

        for (feature, keywords) in keywords_by_feature() {
            if keywords.iter().any(|keyword| search_terms.contains(keyword)) {
                self.upsells.insert(*feature);
            } else {
                self.upsells.remove(feature);
            }
        }
    }

    fn render_feature_upsell_banner(
        &self,
        label: SharedString,
        docs_url: SharedString,
        vim: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let docs_url_button = Button::new("open_docs", "View Documentation")
            .icon(IconName::ArrowUpRight)
            .icon_size(IconSize::Small)
            .icon_position(IconPosition::End)
            .on_click(move |_event, _window, cx| cx.open_url(&docs_url));

        div()
            .pt_4()
            .px_4()
            .child(
                Banner::new()
                    .severity(Severity::Success)
                    .child(Label::new(label).mt_0p5())
                    .map(|this| {
                        if vim {
                            this.action_slot(
                                h_flex()
                                    .gap_1()
                                    .child(docs_url_button)
                                    .child(Divider::vertical().color(ui::DividerColor::Border))
                                    .child(
                                        h_flex().pl_1().gap_1().child(Label::new("Enable Vim mode")).child(
                                            Switch::new(
                                                "enable-vim",
                                                if VimModeSetting::get_global(cx).0 {
                                                    ui::ToggleState::Selected
                                                } else {
                                                    ui::ToggleState::Unselected
                                                },
                                            )
                                            .on_click(cx.listener(
                                                move |this, selection, _, cx| {
                                                    this.update_settings(selection, cx, |setting, value| {
                                                        setting.vim_mode = Some(value)
                                                    });
                                                },
                                            )),
                                        ),
                                    ),
                            )
                        } else {
                            this.action_slot(docs_url_button)
                        }
                    }),
            )
            .into_any_element()
    }

    fn render_feature_upsells(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let mut container = v_flex();

        for feature in &self.upsells {
            let banner = match feature {
                Feature::ExtensionBasedpyright => self.render_feature_upsell_banner(
                    "Basedpyright (Python language server) support is built-in!".into(),
                    "monyacode://docs/languages/python#basedpyright".into(),
                    false,
                    cx,
                ),
                Feature::ExtensionRuff => self.render_feature_upsell_banner(
                    "Ruff (linter for Python) support is built-in!".into(),
                    "monyacode://docs/languages/python#code-formatting--linting".into(),
                    false,
                    cx,
                ),
                Feature::ExtensionTailwind => self.render_feature_upsell_banner(
                    "Tailwind CSS support is built-in!".into(),
                    "monyacode://docs/languages/tailwindcss".into(),
                    false,
                    cx,
                ),
                Feature::ExtensionTy => self.render_feature_upsell_banner(
                    "Ty (Python language server) support is built-in!".into(),
                    "https://zed.dev/docs/languages/python".into(),
                    false,
                    cx,
                ),
                Feature::Git => self.render_feature_upsell_banner(
                    "MonyaCode comes with basic Git support—more features are coming in the future.".into(),
                    "monyacode://docs/git".into(),
                    false,
                    cx,
                ),
                Feature::LanguageBash => self.render_feature_upsell_banner(
                    "Shell support is built-in!".into(),
                    "monyacode://docs/languages/bash".into(),
                    false,
                    cx,
                ),
                Feature::LanguageC => self.render_feature_upsell_banner(
                    "C support is built-in!".into(),
                    "monyacode://docs/languages/c".into(),
                    false,
                    cx,
                ),
                Feature::LanguageCpp => self.render_feature_upsell_banner(
                    "C++ support is built-in!".into(),
                    "monyacode://docs/languages/cpp".into(),
                    false,
                    cx,
                ),
                Feature::LanguageGo => self.render_feature_upsell_banner(
                    "Go support is built-in!".into(),
                    "monyacode://docs/languages/go".into(),
                    false,
                    cx,
                ),
                Feature::LanguagePython => self.render_feature_upsell_banner(
                    "Python support is built-in!".into(),
                    "monyacode://docs/languages/python".into(),
                    false,
                    cx,
                ),
                Feature::LanguageReact => self.render_feature_upsell_banner(
                    "React support is built-in!".into(),
                    "monyacode://docs/languages/typescript".into(),
                    false,
                    cx,
                ),
                Feature::LanguageRust => self.render_feature_upsell_banner(
                    "Rust support is built-in!".into(),
                    "monyacode://docs/languages/rust".into(),
                    false,
                    cx,
                ),
                Feature::LanguageTypescript => self.render_feature_upsell_banner(
                    "Typescript support is built-in!".into(),
                    "monyacode://docs/languages/typescript".into(),
                    false,
                    cx,
                ),
                Feature::OpenIn => self.render_feature_upsell_banner(
                    "MonyaCode supports linking to a source line on GitHub and others.".into(),
                    "monyacode://docs/git#git-integrations".into(),
                    false,
                    cx,
                ),
                Feature::Vim => self.render_feature_upsell_banner(
                    "Vim support is built-in!".into(),
                    "monyacode://docs/vim".into(),
                    true,
                    cx,
                ),
            };
            container = container.child(banner);
        }

        container
    }
}

impl Render for ExtensionsPage {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .size_full()
            .bg(cx.theme().colors().editor_background)
            .child(
                v_flex()
                    .gap_4()
                    .pt_4()
                    .px_4()
                    .bg(cx.theme().colors().editor_background)
                    .child(
                        h_flex()
                            .w_full()
                            .gap_1p5()
                            .justify_between()
                            .child(Headline::new("Extensions").size(HeadlineSize::XLarge))
                            .child(
                                h_flex()
                                    .gap_1p5()
                                    .child(
                                        Button::new("install-extension-from-url", "Install From URL")
                                            .style(ButtonStyle::Outlined)
                                            .size(ButtonSize::Medium)
                                            .on_click(|_event, window, cx| {
                                                window.dispatch_action(Box::new(InstallExtensionFromUrl), cx)
                                            }),
                                    )
                                    .child(
                                        Button::new("install-local-extension", "Install Local")
                                            .style(ButtonStyle::Outlined)
                                            .size(ButtonSize::Medium)
                                            .on_click(|_event, window, cx| {
                                                window.dispatch_action(Box::new(InstallExtensionFromFolder), cx)
                                            }),
                                    ),
                            ),
                    )
                    .child(h_flex().w_full().flex_wrap().gap_2().child(self.render_search(cx))),
            )
            .child(
                h_flex()
                    .id("filter-row")
                    .gap_2()
                    .py_2p5()
                    .px_4()
                    .border_b_1()
                    .border_color(cx.theme().colors().border_variant)
                    .overflow_x_scroll()
                    .child(
                        Button::new("filter-all-categories", "All")
                            .when(self.provides_filter.is_none(), |button| {
                                button.style(ButtonStyle::Filled)
                            })
                            .when(self.provides_filter.is_some(), |button| {
                                button.style(ButtonStyle::Subtle)
                            })
                            .toggle_state(self.provides_filter.is_none())
                            .on_click(cx.listener(|this, _event, _, cx| {
                                this.change_provides_filter(None, cx);
                            })),
                    )
                    .children(ExtensionProvides::iter().filter_map(|provides| {
                        match provides {
                            ExtensionProvides::IndexedDocsProviders => return None,
                            _ => {}
                        }

                        let label = extension_provides_label(provides);
                        let button_id = SharedString::from(format!("filter-category-{}", label));

                        Some(
                            Button::new(button_id, label)
                                .style(if self.provides_filter == Some(provides) {
                                    ButtonStyle::Filled
                                } else {
                                    ButtonStyle::Subtle
                                })
                                .toggle_state(self.provides_filter == Some(provides))
                                .on_click({
                                    cx.listener(move |this, _event, _, cx| {
                                        this.change_provides_filter(Some(provides), cx);
                                    })
                                }),
                        )
                    })),
            )
            .child(self.render_feature_upsells(cx))
            .child(v_flex().px_4().size_full().overflow_y_hidden().map(|this| {
                let count = self.filtered_remote_extension_indices.len() + self.extension_entries.len();

                if count == 0 {
                    this.child(self.render_empty_state(cx)).into_any_element()
                } else {
                    let scroll_handle = &self.list;
                    this.child(
                        uniform_list("entries", count, cx.processor(Self::render_extensions))
                            .flex_grow()
                            .pb_4()
                            .track_scroll(scroll_handle),
                    )
                    .vertical_scrollbar_for(scroll_handle, window, cx)
                    .into_any_element()
                }
            }))
    }
}

impl EventEmitter<ItemEvent> for ExtensionsPage {}

impl Focusable for ExtensionsPage {
    fn focus_handle(&self, cx: &App) -> gpui::FocusHandle {
        self.query_editor.read(cx).focus_handle(cx)
    }
}

impl Item for ExtensionsPage {
    type Event = ItemEvent;

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        "Extensions".into()
    }

    fn show_toolbar(&self) -> bool {
        false
    }

    fn to_item_events(event: &Self::Event, mut f: impl FnMut(workspace::item::ItemEvent)) {
        f(*event)
    }
}

struct GitCloneModal {
    repo_input: Entity<Editor>,
    focus_handle: FocusHandle,
}

impl GitCloneModal {
    pub fn show(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let repo_input = cx.new(|cx| {
            let mut editor = Editor::single_line(window, cx);
            editor.set_placeholder_text("Enter repository URL…", window, cx);
            editor
        });
        let focus_handle = repo_input.focus_handle(cx);

        window.focus(&focus_handle, cx);

        Self {
            repo_input,
            focus_handle,
        }
    }
}

impl Focusable for GitCloneModal {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for GitCloneModal {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme().colors();
        div()
            .elevation_3(cx)
            .w(rems(34.))
            .flex_1()
            .overflow_hidden()
            .child(
                Modal::new("extension_git_clone_modal", None)
                    .header(
                        ModalHeader::new().child(
                            v_flex()
                                .w_full()
                                .pb_1p5()
                                .mb_1()
                                .gap_0p5()
                                .border_b_1()
                                .border_color(theme.border_variant)
                                .child(Label::new("Install Extension From Git Repository")),
                        ),
                    )
                    .section(
                        Section::new()
                            .child(
                                v_flex()
                                    .gap_2p5()
                                    .child(v_flex().gap_1().child(self.repo_input.clone())),
                            )
                            .child(
                                h_flex()
                                    .w_full()
                                    .p_2()
                                    .gap_0p5()
                                    .rounded_b_sm()
                                    .child(
                                        Label::new(
                                            "Clone a repository from a Git forge (like Codeberg, etc.).\nInstallation from source URL requires rustup to be installed.\nUse a public HTTP(S) URL. Git over SSH will not work.",
                                        )
                                        .color(Color::Muted)
                                        .size(LabelSize::Small),
                                    )
                                    .child(
                                        Button::new("learn-more", "Learn More")
                                            .label_size(LabelSize::Small)
                                            .icon(IconName::ArrowUpRight)
                                            .icon_size(IconSize::XSmall)
                                            .on_click(|_, _, cx| {
                                                cx.open_url(
                                                    "https://monyacode-editor.com/docs/extensions/installing-extensions/",
                                                );
                                            }),
                                    ),
                            ),
                    ),
            )
            .on_action(cx.listener(|_, _: &menu::Cancel, _, cx| {
                cx.emit(DismissEvent);
            }))
            .on_action(cx.listener(|this, _: &menu::Confirm, window, cx| {
                let url = this.repo_input.read(cx).text(cx);
                window.dispatch_action(
                    Box::new(InstallExtensionFromGit { url }),
                    cx,
                );
                cx.emit(DismissEvent);
            }))
    }
}

impl EventEmitter<DismissEvent> for GitCloneModal {}

impl ModalView for GitCloneModal {}

/// Returns the set of features provided by the extension.
fn extension_provides(manifest: &ExtensionManifest) -> BTreeSet<ExtensionProvides> {
    let mut provides = BTreeSet::default();
    if !manifest.themes.is_empty() {
        provides.insert(ExtensionProvides::Themes);
    }

    if !manifest.icon_themes.is_empty() {
        provides.insert(ExtensionProvides::IconThemes);
    }

    if !manifest.languages.is_empty() {
        provides.insert(ExtensionProvides::Languages);
    }

    if !manifest.grammars.is_empty() {
        provides.insert(ExtensionProvides::Grammars);
    }

    if !manifest.language_servers.is_empty() {
        provides.insert(ExtensionProvides::LanguageServers);
    }

    if manifest.snippets.is_some() {
        provides.insert(ExtensionProvides::Snippets);
    }

    if !manifest.debug_adapters.is_empty() {
        provides.insert(ExtensionProvides::DebugAdapters);
    }

    provides
}
