use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use file_icons::FileIcons;
use fuzzy::{StringMatch, StringMatchCandidate};
use gpui::{
    Action, AppContext, Context, DismissEvent, Entity, EventEmitter, FocusHandle, Focusable, Modifiers,
    ModifiersChangedEvent, Task, WeakEntity, actions, rems,
};
use picker::{Picker, PickerDelegate};
use postage::{sink::Sink, stream::Stream};
use ui::{
    App, Color, FluentBuilder, HighlightedLabel, Icon, InteractiveElement, IntoElement, LabelCommon, LabelSize,
    ListItem, ListItemSpacing, ParentElement, Render, SharedString, Styled, Toggleable, Window, h_flex, v_flex,
};
use util::ResultExt;
use workspace::{ModalView, OpenOptions, WORKSPACE_DB, Workspace};

const PANEL_WIDTH_REMS: f32 = 34.;

actions!(
    recent_files,
    [
        /// Toggle the recent files picker
        Toggle,
    ]
);

struct RecentFiles {
    picker: Entity<Picker<RecentFilesDelegate>>,
    init_modifiers: Option<Modifiers>,
}

impl ModalView for RecentFiles {}

pub fn init(cx: &mut App) {
    cx.observe_new(RecentFiles::register).detach();
}

impl RecentFiles {
    fn register(workspace: &mut Workspace, _window: Option<&mut Window>, _: &mut Context<Workspace>) {
        workspace.register_action(|workspace, _: &Toggle, window, cx| {
            let Some(recent_files) = workspace.active_modal::<Self>(cx) else {
                Self::open(workspace, window, cx);
                return;
            };

            recent_files.update(cx, |recent_files, cx| {
                recent_files.picker.update(cx, |picker, cx| {
                    picker.cycle_selection(window, cx);
                });
            });
        });
    }

    fn open(workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
        let weak_workspace = workspace.weak_handle();
        let initial_modifiers = window.modifiers();
        workspace.toggle_modal(window, cx, |window, cx| {
            let delegate = RecentFilesDelegate::new(cx.entity().downgrade(), weak_workspace, window, cx);
            let mut recent_files = RecentFiles::new(delegate, window, cx);
            if initial_modifiers.modified() {
                recent_files.init_modifiers = Some(initial_modifiers);
            }
            recent_files
        });
    }

    fn new(delegate: RecentFilesDelegate, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let picker = cx.new(|cx| Picker::list(delegate, window, cx));
        Self {
            picker,
            init_modifiers: None,
        }
    }

    fn handle_modifiers_changed(&mut self, event: &ModifiersChangedEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some(init_modifiers) = self.init_modifiers else {
            return;
        };
        if !event.modified() || !init_modifiers.is_subset_of(event) {
            self.init_modifiers = None;
            if self.picker.read(cx).delegate.matches.is_empty() {
                cx.emit(DismissEvent)
            } else {
                window.dispatch_action(menu::Confirm.boxed_clone(), cx);
            }
        }
    }
}

impl EventEmitter<DismissEvent> for RecentFiles {}

impl Focusable for RecentFiles {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.picker.focus_handle(cx)
    }
}

impl Render for RecentFiles {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .key_context("RecentFiles")
            .w(rems(PANEL_WIDTH_REMS))
            .on_modifiers_changed(cx.listener(Self::handle_modifiers_changed))
            .child(self.picker.clone())
    }
}

struct RecentFilesDelegate {
    recent_files: WeakEntity<RecentFiles>,
    selected_index: usize,
    workspace: WeakEntity<Workspace>,
    paths: Vec<PathBuf>,
    matches: Vec<StringMatch>,
    updating_matches: Option<(Task<()>, postage::dispatch::Receiver<Vec<StringMatch>>)>,
}

impl RecentFilesDelegate {
    fn new(
        recent_files: WeakEntity<RecentFiles>,
        workspace: WeakEntity<Workspace>,
        _window: &mut Window,
        cx: &mut Context<RecentFiles>,
    ) -> Self {
        let this = Self {
            recent_files,
            workspace,
            paths: Vec::new(),
            matches: Vec::new(),
            selected_index: 0,
            updating_matches: None,
        };
        cx.spawn(async move |this, cx| {
            let paths = WORKSPACE_DB.recent_files(100).log_err();
            if let Some(paths) = paths {
                this.update(cx, |this, cx| {
                    this.picker.update(cx, |picker, cx| {
                        picker.delegate.set_paths(paths, cx);
                    });
                })
                .log_err();
            }
        })
        .detach();
        this
    }

    fn set_paths(&mut self, paths: Vec<PathBuf>, cx: &mut Context<Picker<Self>>) {
        self.paths = paths.into_iter().filter(|path| path.exists()).collect();
        self.matches_from_paths(|(idx, path)| StringMatch {
            candidate_id: idx,
            string: homify(path),
            positions: Vec::new(),
            score: 0.0,
        });
        cx.notify();
    }

    fn matches_from_paths(&mut self, f: impl Fn((usize, &PathBuf)) -> StringMatch) {
        self.matches = self.paths.iter().enumerate().map(f).collect();
    }

    fn icon_for_file(&self, path: &Path, cx: &App) -> Option<Icon> {
        let file_name = path.file_name()?;
        let icon = FileIcons::get_icon(file_name.as_ref(), cx)?;
        Some(Icon::from_path(icon).color(Color::Muted))
    }

    fn labels_for_match(
        &self,
        _path_buf: &PathBuf,
        string_match: &StringMatch,
    ) -> (HighlightedLabel, Option<HighlightedLabel>) {
        let (left, right) = string_match.split_at_from_end(if cfg!(windows) { "\\" } else { "/" });
        if let Some(right) = right {
            (
                HighlightedLabel::new(right.string.clone(), right.positions),
                Some(
                    HighlightedLabel::new(left.string.clone(), left.positions)
                        .size(LabelSize::Small)
                        .color(Color::Muted),
                ),
            )
        } else {
            (HighlightedLabel::new(left.string.clone(), left.positions), None)
        }
    }
}

impl PickerDelegate for RecentFilesDelegate {
    type ListItem = ListItem;

    fn placeholder_text(&self, _: &mut Window, _: &mut App) -> Arc<str> {
        "Open recent files...".into()
    }

    fn no_matches_text(&self, _window: &mut Window, _cx: &mut App) -> Option<SharedString> {
        Some("No recent files".into())
    }

    fn match_count(&self) -> usize {
        self.matches.len()
    }

    fn selected_index(&self) -> usize {
        self.selected_index
    }

    fn update_matches(&mut self, query: String, window: &mut Window, cx: &mut ui::Context<Picker<Self>>) -> Task<()> {
        if query.is_empty() {
            self.matches_from_paths(|(idx, path)| StringMatch {
                candidate_id: idx,
                string: homify(path),
                positions: Vec::new(),
                score: 0.0,
            });
            self.selected_index = 0;
            return Task::ready(());
        }

        let (mut tx, mut rx) = postage::dispatch::channel(1);

        let task = cx.background_spawn({
            let paths = self.paths.clone();
            let executor = cx.background_executor().clone();
            async move {
                let candidates = paths
                    .iter()
                    .filter(|path| path.exists())
                    .enumerate()
                    .map(|(idx, path)| StringMatchCandidate::new(idx, homify(path).as_str()))
                    .collect::<Vec<_>>();
                let matches =
                    fuzzy::match_strings(&candidates, &query, true, true, 10000, &Default::default(), executor).await;

                tx.send(matches).await.log_err();
            }
        });

        self.updating_matches = Some((task, rx.clone()));

        cx.spawn_in(window, async move |picker, cx| {
            let Some(matches) = rx.recv().await else {
                return;
            };

            picker
                .update(cx, |picker, cx| {
                    picker.delegate.matches = matches;
                    picker.delegate.selected_index = 0;
                    cx.notify();
                })
                .log_err();
        })
    }

    fn set_selected_index(&mut self, ix: usize, _window: &mut Window, _cx: &mut Context<Picker<Self>>) {
        self.selected_index = ix;
    }

    fn confirm(&mut self, _secondary: bool, window: &mut Window, cx: &mut ui::Context<Picker<Self>>) {
        let Some(selected_match) = self.matches.get(self.selected_index()).cloned() else {
            return;
        };

        let path = self.paths[selected_match.candidate_id].clone();

        let Some(workspace) = self.workspace.upgrade() else {
            return;
        };

        workspace.update(cx, |workspace, cx| {
            workspace
                .open_abs_path(path, OpenOptions::default(), window, cx)
                .detach_and_log_err(cx);
        });
        self.dismissed(window, cx);
    }
    fn dismissed(&mut self, _window: &mut Window, cx: &mut ui::Context<Picker<Self>>) {
        self.recent_files.update(cx, |_, cx| cx.emit(DismissEvent)).log_err();
    }

    fn render_match(
        &self,
        ix: usize,
        selected: bool,
        _window: &mut Window,
        cx: &mut ui::Context<Picker<Self>>,
    ) -> Option<Self::ListItem> {
        let m = self.matches.get(ix)?;
        let p = self.paths.get(m.candidate_id)?;
        let file_icon = self.icon_for_file(p.as_path(), cx);
        let (left, right) = self.labels_for_match(p, m);
        Some(
            ListItem::new(ix)
                .spacing(ListItemSpacing::Sparse)
                .start_slot::<Icon>(file_icon)
                .inset(true)
                .toggle_state(selected)
                .child(
                    h_flex()
                        .gap_2()
                        .py_px()
                        .when(right.is_some(), |this| this.child(left).child(right.unwrap())),
                ),
        )
    }
}

fn homify(path: &PathBuf) -> String {
    let mut s = path.to_string_lossy().to_string();
    let user_home_path = util::paths::home_dir().to_string_lossy();
    if !user_home_path.is_empty() && s.starts_with(&*user_home_path) {
        s.replace_range(0..user_home_path.len(), "~");
    }
    s
}
