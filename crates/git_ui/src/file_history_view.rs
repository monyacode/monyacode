use anyhow::Result;
use git::repository::{CommitHistory, CommitHistoryEntry, RepoPath};
use gpui::{
    AnyElement, AnyEntity, App, Context, Entity, EventEmitter, FocusHandle, Focusable, IntoElement, Render,
    ScrollStrategy, Task, UniformListScrollHandle, WeakEntity, Window, actions, uniform_list,
};
use project::{
    Project, ProjectPath,
    git_store::{GitStore, Repository},
};
use std::any::{Any, TypeId};

use time::OffsetDateTime;
use ui::{Chip, Divider, ListItem, WithScrollbar, prelude::*, render_avatar};
use util::ResultExt;
use workspace::{
    Item, Workspace,
    item::{ItemEvent, SaveOptions},
};

use crate::commit_view::CommitView;

actions!(git, [ShowHistory]);

pub fn init(cx: &mut App) {
    cx.observe_new(|workspace: &mut Workspace, _window, _cx| {
        workspace.register_action(FileHistoryView::show_history);
    })
    .detach();
}

const PAGE_SIZE: usize = 50;

pub struct FileHistoryView {
    history: CommitHistory,
    repository: WeakEntity<Repository>,
    git_store: WeakEntity<GitStore>,
    workspace: WeakEntity<Workspace>,
    selected_entry: Option<usize>,
    scroll_handle: UniformListScrollHandle,
    focus_handle: FocusHandle,
    loading_more: bool,
    has_more: bool,
}

impl FileHistoryView {
    pub fn show_history(workspace: &mut Workspace, _: &ShowHistory, window: &mut Window, cx: &mut Context<Workspace>) {
        let project = workspace.project();
        let git_store = project.read(cx).git_store();
        if let Some(repo) = git_store.read(cx).active_repository() {
            Self::open(
                None,
                git_store.downgrade(),
                repo.downgrade(),
                workspace.weak_handle(),
                window,
                cx,
            );
        }
    }

    pub fn open(
        path: Option<RepoPath>,
        git_store: WeakEntity<GitStore>,
        repo: WeakEntity<Repository>,
        workspace: WeakEntity<Workspace>,
        window: &mut Window,
        cx: &mut App,
    ) {
        let file_history_task = git_store
            .update(cx, |git_store, cx| {
                repo.upgrade()
                    .map(|repo| git_store.commit_history_paginated(&repo, path.clone(), 0, Some(PAGE_SIZE), cx))
            })
            .ok()
            .flatten();

        window
            .spawn(cx, async move |cx| {
                let file_history = file_history_task?.await.log_err()?;
                let repo = repo.upgrade()?;

                workspace
                    .update_in(cx, |workspace, window, cx| {
                        let view = cx.new(|cx| {
                            FileHistoryView::new(
                                file_history,
                                git_store.clone(),
                                repo.clone(),
                                workspace.weak_handle(),
                                cx,
                            )
                        });

                        let pane = workspace.active_pane();
                        pane.update(cx, |pane, cx| {
                            let ix = pane.items().position(|item| {
                                let view = item.downcast::<FileHistoryView>();
                                view.is_some_and(|v| v.read(cx).history.path == path)
                            });
                            if let Some(ix) = ix {
                                pane.activate_item(ix, true, true, window, cx);
                            } else {
                                pane.add_item(Box::new(view), true, true, None, window, cx);
                            }
                        })
                    })
                    .log_err()
            })
            .detach();
    }

    fn new(
        history: CommitHistory,
        git_store: WeakEntity<GitStore>,
        repository: Entity<Repository>,
        workspace: WeakEntity<Workspace>,
        cx: &mut Context<Self>,
    ) -> Self {
        let focus_handle = cx.focus_handle();
        let scroll_handle = UniformListScrollHandle::new();
        let has_more = history.entries.len() >= PAGE_SIZE;

        Self {
            history,
            git_store,
            repository: repository.downgrade(),
            workspace,
            selected_entry: None,
            scroll_handle,
            focus_handle,
            loading_more: false,
            has_more,
        }
    }

    fn load_more(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.loading_more || !self.has_more {
            return;
        }

        self.loading_more = true;
        cx.notify();

        let current_count = self.history.entries.len();
        let path = self.history.path.clone();
        let git_store = self.git_store.clone();
        let repo = self.repository.clone();

        let this = cx.weak_entity();
        let task = window.spawn(cx, async move |cx| {
            let file_history_task = git_store
                .update(cx, |git_store, cx| {
                    repo.upgrade()
                        .map(|repo| git_store.commit_history_paginated(&repo, path, current_count, Some(PAGE_SIZE), cx))
                })
                .ok()
                .flatten();

            if let Some(task) = file_history_task {
                if let Ok(more_history) = task.await {
                    this.update(cx, |this, cx| {
                        this.loading_more = false;
                        this.has_more = more_history.entries.len() >= PAGE_SIZE;
                        this.history.entries.extend(more_history.entries);
                        cx.notify();
                    })
                    .ok();
                }
            }
        });

        task.detach();
    }

    fn select_next(&mut self, _: &menu::SelectNext, _: &mut Window, cx: &mut Context<Self>) {
        let entry_count = self.history.entries.len();
        let ix = match self.selected_entry {
            _ if entry_count == 0 => None,
            None => Some(0),
            Some(ix) => {
                if ix == entry_count - 1 {
                    Some(0)
                } else {
                    Some(ix + 1)
                }
            }
        };
        self.select_ix(ix, cx);
    }

    fn select_previous(&mut self, _: &menu::SelectPrevious, _: &mut Window, cx: &mut Context<Self>) {
        let entry_count = self.history.entries.len();
        let ix = match self.selected_entry {
            _ if entry_count == 0 => None,
            None => Some(entry_count - 1),
            Some(ix) => {
                if ix == 0 {
                    Some(entry_count - 1)
                } else {
                    Some(ix - 1)
                }
            }
        };
        self.select_ix(ix, cx);
    }

    fn select_first(&mut self, _: &menu::SelectFirst, _: &mut Window, cx: &mut Context<Self>) {
        let entry_count = self.history.entries.len();
        let ix = if entry_count != 0 { Some(0) } else { None };
        self.select_ix(ix, cx);
    }

    fn select_last(&mut self, _: &menu::SelectLast, _: &mut Window, cx: &mut Context<Self>) {
        let entry_count = self.history.entries.len();
        let ix = if entry_count != 0 { Some(entry_count - 1) } else { None };
        self.select_ix(ix, cx);
    }

    fn select_ix(&mut self, ix: Option<usize>, cx: &mut Context<Self>) {
        self.selected_entry = ix;
        if let Some(ix) = ix {
            self.scroll_handle.scroll_to_item(ix, ScrollStrategy::Top);
        }
        cx.notify();
    }

    fn confirm(&mut self, _: &menu::Confirm, window: &mut Window, cx: &mut Context<Self>) {
        self.open_commit_view(window, cx);
    }

    fn open_commit_view(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(entry) = self.selected_entry.and_then(|ix| self.history.entries.get(ix)) else {
            return;
        };

        if let Some(repo) = self.repository.upgrade() {
            let sha_str = entry.sha.to_string();
            CommitView::open(
                sha_str,
                repo.downgrade(),
                self.workspace.clone(),
                None,
                self.history.path.clone(),
                crate::commit_view::OpenMode::Preview,
                window,
                cx,
            );
        }
    }

    fn render_commit_entry(
        &self,
        ix: usize,
        entry: &CommitHistoryEntry,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let pr_number = entry
            .subject
            .rfind("(#")
            .and_then(|start| {
                let rest = &entry.subject[start + 2..];
                rest.find(')').and_then(|end| rest[..end].parse::<u32>().ok())
            })
            .map(|num| format!("#{}", num))
            .unwrap_or_else(|| {
                if entry.sha.len() >= 7 {
                    entry.sha[..7].to_string()
                } else {
                    entry.sha.to_string()
                }
            });

        let commit_time =
            OffsetDateTime::from_unix_timestamp(entry.commit_timestamp).unwrap_or_else(|_| OffsetDateTime::UNIX_EPOCH);
        let relative_timestamp = time_format::format_localized_timestamp(
            commit_time,
            OffsetDateTime::now_utc(),
            time::UtcOffset::current_local_offset().unwrap_or(time::UtcOffset::UTC),
            time_format::TimestampFormat::Relative,
        );

        ListItem::new(("commit", ix))
            .toggle_state(Some(ix) == self.selected_entry)
            .child(
                h_flex()
                    .h_8()
                    .w_full()
                    .p_1()
                    .gap_2()
                    .child(
                        div().min_w(rems(8.)).flex_row().text_right().child(
                            Label::new(relative_timestamp)
                                .size(LabelSize::Small)
                                .color(Color::Muted)
                                .italic()
                                .truncate(),
                        ),
                    )
                    .child(
                        h_flex().min_w_0().w_full().justify_between().child(
                            Label::new(&entry.subject)
                                .size(LabelSize::Small)
                                .color(Color::Default)
                                .truncate(),
                        ),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .ml_auto()
                            .child(render_avatar(rems(1.25), window, cx))
                            .child(
                                div().w_full().child(
                                    Label::new(entry.author_name.clone())
                                        .size(LabelSize::Small)
                                        .color(Color::Muted)
                                        .truncate(),
                                ),
                            )
                            .child(h_flex().flex_none().child(Chip::new(pr_number))),
                    ),
            )
            .on_click(cx.listener(move |this, _, window, cx| {
                this.selected_entry = Some(ix);
                cx.notify();

                this.open_commit_view(window, cx);
            }))
            .into_any_element()
    }

    fn title(&self) -> SharedString {
        if let Some(path) = self.history.path.as_ref()
            && !path.is_empty()
        {
            path.as_unix_str().to_string().into()
        } else {
            "Git History".into()
        }
    }
}

impl EventEmitter<ItemEvent> for FileHistoryView {}

impl Focusable for FileHistoryView {
    fn focus_handle(&self, _cx: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for FileHistoryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let entry_count = self.history.entries.len();

        v_flex()
            .id("file_history_view")
            .key_context("FileHistoryView")
            .track_focus(&self.focus_handle)
            .on_action(cx.listener(Self::select_next))
            .on_action(cx.listener(Self::select_previous))
            .on_action(cx.listener(Self::select_first))
            .on_action(cx.listener(Self::select_last))
            .on_action(cx.listener(Self::confirm))
            .size_full()
            .bg(cx.theme().colors().editor_background)
            .child(
                h_flex()
                    .h(rems(2.5))
                    .pl_3()
                    .pr_2()
                    .justify_between()
                    .border_b_1()
                    .border_color(cx.theme().colors().border_variant)
                    .child(Label::new(self.title()).color(Color::Muted).buffer_font(cx))
                    .child(
                        h_flex()
                            .gap_1p5()
                            .child(
                                Label::new(format!("{} commits", entry_count))
                                    .size(LabelSize::Small)
                                    .color(Color::Muted)
                                    .when(self.has_more, |this| this.mr_1()),
                            )
                            .when(self.has_more, |this| {
                                this.child(Divider::vertical()).child(
                                    Button::new("load-more", "Load More")
                                        .disabled(self.loading_more)
                                        .label_size(LabelSize::Small)
                                        .icon(IconName::ArrowCircle)
                                        .icon_size(IconSize::Small)
                                        .icon_color(Color::Muted)
                                        .icon_position(IconPosition::Start)
                                        .on_click(cx.listener(|this, _, window, cx| {
                                            this.load_more(window, cx);
                                        })),
                                )
                            }),
                    ),
            )
            .child(
                v_flex()
                    .flex_1()
                    .size_full()
                    .child({
                        let view = cx.weak_entity();
                        uniform_list("file-history-list", entry_count, move |range, window, cx| {
                            let Some(view) = view.upgrade() else {
                                return Vec::new();
                            };
                            view.update(cx, |this, cx| {
                                let mut items = Vec::with_capacity(range.end - range.start);
                                for ix in range {
                                    if let Some(entry) = this.history.entries.get(ix) {
                                        items.push(this.render_commit_entry(ix, entry, window, cx));
                                    }
                                }
                                items
                            })
                        })
                        .flex_1()
                        .size_full()
                        .track_scroll(&self.scroll_handle)
                    })
                    .vertical_scrollbar_for(&self.scroll_handle, window, cx),
            )
    }
}

impl Item for FileHistoryView {
    type Event = ItemEvent;

    fn to_item_events(event: &Self::Event, mut f: impl FnMut(ItemEvent)) {
        f(*event)
    }

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        self.title()
    }

    fn tab_tooltip_text(&self, _cx: &App) -> Option<SharedString> {
        Some(
            format!(
                "Git history for {}",
                self.history
                    .path
                    .as_ref()
                    .map(|path| if !path.is_empty() {
                        path.as_unix_str()
                    } else {
                        "project"
                    })
                    .unwrap_or("project")
            )
            .into(),
        )
    }

    fn tab_icon(&self, _window: &Window, _cx: &App) -> Option<Icon> {
        Some(Icon::new(IconName::GitBranch))
    }

    fn clone_on_split(
        &self,
        _workspace_id: Option<workspace::WorkspaceId>,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) -> Task<Option<Entity<Self>>> {
        Task::ready(None)
    }

    fn navigate(&mut self, _: Box<dyn Any>, _window: &mut Window, _: &mut Context<Self>) -> bool {
        false
    }

    fn deactivated(&mut self, _window: &mut Window, _: &mut Context<Self>) {}

    fn can_save(&self, _: &App) -> bool {
        false
    }

    fn save(
        &mut self,
        _options: SaveOptions,
        _project: Entity<Project>,
        _window: &mut Window,
        _: &mut Context<Self>,
    ) -> Task<Result<()>> {
        Task::ready(Ok(()))
    }

    fn save_as(
        &mut self,
        _project: Entity<Project>,
        _path: ProjectPath,
        _window: &mut Window,
        _: &mut Context<Self>,
    ) -> Task<Result<()>> {
        Task::ready(Ok(()))
    }

    fn reload(&mut self, _project: Entity<Project>, _window: &mut Window, _: &mut Context<Self>) -> Task<Result<()>> {
        Task::ready(Ok(()))
    }

    fn is_dirty(&self, _: &App) -> bool {
        false
    }

    fn has_conflict(&self, _: &App) -> bool {
        false
    }

    fn breadcrumbs(&self, _theme: &theme::Theme, _cx: &App) -> Option<Vec<workspace::item::BreadcrumbText>> {
        None
    }

    fn added_to_workspace(&mut self, _workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Self>) {
        window.focus(&self.focus_handle, cx);
    }

    fn show_toolbar(&self) -> bool {
        true
    }

    fn pixel_position_of_cursor(&self, _: &App) -> Option<gpui::Point<gpui::Pixels>> {
        None
    }

    fn set_nav_history(&mut self, _: workspace::ItemNavHistory, _window: &mut Window, _: &mut Context<Self>) {}

    fn act_as_type<'a>(&'a self, type_id: TypeId, self_handle: &'a Entity<Self>, _: &'a App) -> Option<AnyEntity> {
        if type_id == TypeId::of::<Self>() {
            Some(self_handle.clone().into())
        } else {
            None
        }
    }
}
