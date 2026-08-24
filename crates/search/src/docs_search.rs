use assets::Docs;
use clock::ReplicaId;
use collections::{HashMap, HashSet};
use fs;
use futures::StreamExt;
use gpui::AssetSource;
use language::{Capability, DiskState, LocalFile};
use smol::channel::unbounded;
use std::cell::RefCell;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::pin::pin;
use std::sync::Arc;
use std::time::Duration;
use text::BufferId;
use util::paths::PathStyle;
use util::rel_path::RelPath;
use worktree::WorktreeId;

use crate::common::{
    BottomResizeDrag, DEFAULT_RESULTS_HEIGHT, DragPreview, MAX_PREVIEW_HEIGHT, MIN_PANEL_HEIGHT, ResizeDrag,
    SEARCH_DEBOUNCE_MS, SearchDrag, SearchMatch,
};
use crate::{
    SearchOption, SearchOptions, SelectNextMatch, SelectPreviousMatch, ToggleCaseSensitive, ToggleIncludeIgnored,
    ToggleRegex, ToggleWholeWord,
};
pub use app_actions::docs_search::Toggle;
use editor::EditorSettings;
use editor::{Editor, EditorEvent, EditorMode, RowHighlightOptions};
use gpui::{
    Action, App, AsyncApp, Context, DismissEvent, DragMoveEvent, Entity, EventEmitter, FocusHandle, Focusable, Global,
    HighlightStyle, KeyContext, ParentElement, Render, Styled, StyledText, Subscription, Task, Window, actions, px,
    relative,
};
use language::Buffer;
use language::language_settings::SoftWrap;
use menu;
use multi_buffer::{ExcerptRange, MultiBuffer};
use picker::{Picker, PickerDelegate, PickerEditorPosition};
use project::SearchResults;
use project::search::{SearchQuery, SearchResult};
use project::{Project, ProjectPath};
use settings::Settings;
use text::{Anchor, Point};
use theme::ActiveTheme;
use ui::Divider;
use ui::{IconButton, IconName, ListItem, ListItemSpacing, Tooltip, prelude::*};
use util::{ResultExt, paths::PathMatcher};
use workspace::{ModalView, Workspace};

actions!(docs_search, [ToggleFilters]);

struct DocFile {
    path: Arc<RelPath>,
}

impl DocFile {
    fn new(path: String) -> Arc<Self> {
        let rel_path = RelPath::new(Path::new(&path), PathStyle::Posix).unwrap().into_owned();
        Arc::new(Self {
            path: Arc::from(rel_path.as_ref()),
        })
    }
}

impl language::File for DocFile {
    fn as_local(&self) -> Option<&dyn LocalFile> {
        None
    }

    fn disk_state(&self) -> DiskState {
        DiskState::Present {
            mtime: fs::MTime::from_seconds_and_nanos(0, 0),
            size: 0,
        }
    }

    fn path(&self) -> &Arc<RelPath> {
        &self.path
    }

    fn full_path(&self, _cx: &App) -> PathBuf {
        self.path.as_std_path().to_path_buf()
    }

    fn path_style(&self, _cx: &App) -> PathStyle {
        PathStyle::Posix
    }

    fn file_name<'a>(&'a self, _cx: &'a App) -> &'a str {
        self.path.file_name().unwrap_or("index.md")
    }

    fn worktree_id(&self, _cx: &App) -> WorktreeId {
        WorktreeId::from_usize(0)
    }

    fn to_proto(&self, _cx: &App) -> rpc::proto::File {
        rpc::proto::File {
            worktree_id: 0,
            path: self.path.as_std_path().display().to_string(),
            ..Default::default()
        }
    }

    fn is_private(&self) -> bool {
        false
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
enum InputPanel {
    Query,
    Include,
    Exclude,
}

struct SearchMatchHighlight;
struct SearchMatchLineHighlight;

/// Global state for storing the recent search query.
struct DocsSearchState {
    last_query: Option<String>,
}

impl Global for DocsSearchState {}

fn get_recent_query(cx: &App) -> Option<String> {
    cx.try_global::<DocsSearchState>()
        .and_then(|state| state.last_query.clone())
}

fn save_recent_query(query: &str, cx: &mut App) {
    cx.set_global(DocsSearchState {
        last_query: Some(query.to_string()),
    });
}

/// Initialize the docs_search module.
pub fn init(cx: &mut App) {
    cx.observe_new(DocsSearch::register).detach();
}

pub struct DocsSearch {
    picker: Entity<Picker<DocsSearchDelegate>>,
    preview_editor: Entity<Editor>,
    focus_handle: FocusHandle,
    offset: gpui::Point<Pixels>,
    results_height: Pixels,
    preview_height: Pixels,
    _subscriptions: Vec<Subscription>,
    _autosave_task: Option<Task<()>>,
}

impl ModalView for DocsSearch {}

impl EventEmitter<DismissEvent> for DocsSearch {}

impl Focusable for DocsSearch {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.picker.read(cx).focus_handle(cx)
    }
}

impl DocsSearch {
    fn register(workspace: &mut Workspace, _window: Option<&mut Window>, _: &mut Context<Workspace>) {
        workspace.register_action(|workspace, _: &Toggle, window, cx| {
            let project = workspace.project().clone();
            workspace.toggle_modal(window, cx, |window, cx| DocsSearch::new(project, window, cx));
        });
    }

    fn new(project: Entity<Project>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let capability = project.read(cx).capability();
        let preview_editor = cx.new(|cx| {
            let multi_buffer = cx.new(|_| MultiBuffer::without_headers(capability));
            let editor = Editor::for_multibuffer(multi_buffer, Some(project.clone()), window, cx);
            editor
        });

        let focus_handle = cx.focus_handle();

        let delegate = DocsSearchDelegate::new(project, preview_editor.clone(), get_recent_query(cx), cx);
        let picker = cx.new(|cx| {
            Picker::uniform_list(delegate, window, cx)
                .modal(false)
                .show_scrollbar(true)
        });

        let subscriptions = vec![
            cx.subscribe_in(&picker, window, |_, _, _: &DismissEvent, _, cx| {
                cx.emit(DismissEvent);
            }),
            cx.on_focus_out(&focus_handle, window, |this, _, window, cx| {
                if window.is_window_active() && !this.focus_handle.contains_focused(window, cx) {
                    cx.emit(DismissEvent);
                }
            }),
            cx.subscribe_in(&preview_editor, window, |this, _, event: &EditorEvent, window, cx| {
                if let EditorEvent::Edited { .. } = event {
                    this._autosave_task = Some(cx.spawn_in(window, async move |this, cx| {
                        cx.background_executor().timer(Duration::from_millis(500)).await;

                        this.update_in(cx, |this, _window, cx| {
                            let delegate = &this.picker.read(cx).delegate;
                            if let Some(m) = delegate.matches.get(delegate.selected_index) {
                                let buffer = m.buffer.clone();
                                let project = delegate.project.clone();
                                let mut buffers = HashSet::default();
                                buffers.insert(buffer);
                                project
                                    .update(cx, |p, cx| p.save_buffers(buffers, cx))
                                    .detach_and_log_err(cx);
                            }
                        })
                        .log_err();
                    }));
                }
            }),
        ];

        Self {
            picker,
            preview_editor,
            focus_handle,
            offset: gpui::Point::default(),
            results_height: px(DEFAULT_RESULTS_HEIGHT),
            preview_height: px(crate::common::DEFAULT_PREVIEW_HEIGHT),
            _subscriptions: subscriptions,
            _autosave_task: None,
        }
    }
}

impl Render for DocsSearch {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let modal_width = rems(50.).to_pixels(window.rem_size());

        let delegate = &self.picker.read(cx).delegate;
        let match_count = delegate.matches.len();
        let file_count = delegate.matches.iter().map(|m| &m.path).collect::<HashSet<_>>().len();
        let search_in_progress = delegate.search_in_progress;
        let selected_index = delegate.selected_index;

        let has_matches = match_count > 0;

        let focus_handle = self.focus_handle.clone();

        let mut key_context = KeyContext::new_with_defaults();
        key_context.add("DocsSearch");

        v_flex()
            .key_context(key_context)
            .id("docs-search")
            .track_focus(&focus_handle)
            .on_action(cx.listener(|_, _: &menu::Cancel, _, cx| {
                cx.emit(DismissEvent);
            }))
            .on_action(cx.listener(|this, _: &ToggleCaseSensitive, window, cx| {
                this.picker.update(cx, |picker, cx| {
                    picker.delegate.search_options.toggle(SearchOptions::CASE_SENSITIVE);
                    picker.refresh(window, cx);
                });
            }))
            .on_action(cx.listener(|this, _: &ToggleWholeWord, window, cx| {
                this.picker.update(cx, |picker, cx| {
                    picker.delegate.search_options.toggle(SearchOptions::WHOLE_WORD);
                    picker.refresh(window, cx);
                });
            }))
            .on_action(cx.listener(|this, _: &ToggleIncludeIgnored, window, cx| {
                this.picker.update(cx, |picker, cx| {
                    picker.delegate.search_options.toggle(SearchOptions::INCLUDE_IGNORED);
                    picker.refresh(window, cx);
                });
            }))
            .on_action(cx.listener(|this, _: &ToggleRegex, window, cx| {
                this.picker.update(cx, |picker, cx| {
                    picker.delegate.search_options.toggle(SearchOptions::REGEX);
                    picker.refresh(window, cx);
                });
            }))
            .on_action(cx.listener(|this, _: &SelectNextMatch, window, cx| {
                this.picker.update(cx, |picker, cx| {
                    let match_count = picker.delegate.matches.len();
                    if match_count > 0 {
                        let new_index = (picker.delegate.selected_index + 1) % match_count;
                        picker.set_selected_index(new_index, None, true, window, cx);
                    }
                });
            }))
            .on_action(cx.listener(|this, _: &SelectPreviousMatch, window, cx| {
                this.picker.update(cx, |picker, cx| {
                    let match_count = picker.delegate.matches.len();
                    if match_count > 0 {
                        let new_index = if picker.delegate.selected_index == 0 {
                            match_count - 1
                        } else {
                            picker.delegate.selected_index - 1
                        };
                        picker.set_selected_index(new_index, None, true, window, cx);
                    }
                });
            }))
            .m_4()
            .relative()
            .top(self.offset.y)
            .left(self.offset.x)
            .w(modal_width)
            .bg(cx.theme().colors().elevated_surface_background)
            .border_1()
            .border_color(cx.theme().colors().border)
            .rounded_lg()
            .shadow_lg()
            .on_drag(
                SearchDrag {
                    mouse_start: window.mouse_position(),
                    offset_start: self.offset,
                },
                |_, _, _, cx| cx.new(|_| DragPreview),
            )
            .on_drag_move::<SearchDrag>(cx.listener(|this, event: &DragMoveEvent<SearchDrag>, _window, cx| {
                let drag = event.drag(cx);
                this.offset = drag.offset_start + (event.event.position - drag.mouse_start);
                cx.notify();
            }))
            .child(
                h_flex()
                    .px_4()
                    .py_2()
                    .border_b_1()
                    .border_color(cx.theme().colors().border)
                    .justify_between()
                    .child(
                        h_flex()
                            .gap_2()
                            .items_center()
                            .child(Label::new("Search Documentation").size(LabelSize::Default))
                            .when(search_in_progress, |this| {
                                this.child(
                                    Label::new(format!("Searching... {} matches in {} files", match_count, file_count))
                                        .color(Color::Muted)
                                        .size(LabelSize::Small),
                                )
                            })
                            .when(!search_in_progress && has_matches, |this| {
                                this.child(
                                    Label::new(format!("{} matches in {} files", match_count, file_count))
                                        .color(Color::Muted)
                                        .size(LabelSize::Small),
                                )
                            }),
                    )
                    .child(
                        h_flex()
                            .gap_1()
                            .items_center()
                            .child({
                                let focus_handle = self.picker.focus_handle(cx);
                                IconButton::new("select-prev-match", IconName::ChevronLeft)
                                    .size(ButtonSize::Compact)
                                    .tooltip(move |_window, cx| {
                                        Tooltip::for_action_in(
                                            "Previous Match",
                                            &SelectPreviousMatch,
                                            &focus_handle,
                                            cx,
                                        )
                                    })
                                    .on_click(|_, window, cx| {
                                        window.dispatch_action(SelectPreviousMatch.boxed_clone(), cx);
                                    })
                            })
                            .child({
                                let focus_handle = self.picker.focus_handle(cx);
                                IconButton::new("select-next-match", IconName::ChevronRight)
                                    .size(ButtonSize::Compact)
                                    .tooltip(move |_window, cx| {
                                        Tooltip::for_action_in("Next Match", &SelectNextMatch, &focus_handle, cx)
                                    })
                                    .on_click(|_, window, cx| {
                                        window.dispatch_action(SelectNextMatch.boxed_clone(), cx);
                                    })
                            })
                            .when(match_count > 0, |this| {
                                this.child(
                                    Label::new(format!("{}/{}", selected_index + 1, match_count))
                                        .size(LabelSize::Small)
                                        .color(Color::Muted),
                                )
                            }),
                    ),
            )
            .child(
                div()
                    .h(self.results_height)
                    .overflow_hidden()
                    .child(self.picker.clone()),
            )
            .when(has_matches, |this| {
                let delegate = &self.picker.read(cx).delegate;
                let selected_match = delegate.matches.get(delegate.selected_index);

                let preview_header = selected_match.map(|m| {
                    let path = &m.path.path;
                    let file_name = path.file_name().map(|name| name.to_string()).unwrap_or_default();
                    let directory = path
                        .parent()
                        .map(|path| path.as_std_path().to_string_lossy().to_string())
                        .unwrap_or_default();

                    h_flex()
                        .px_2()
                        .py_1()
                        .gap_2()
                        .border_b_1()
                        .border_color(cx.theme().colors().border)
                        .bg(cx.theme().colors().editor_background)
                        .justify_between()
                        .child(
                            h_flex()
                                .gap_2()
                                .child(Label::new(file_name).size(LabelSize::Small))
                                .child(Label::new(directory).size(LabelSize::Small).color(Color::Muted)),
                        )
                });

                // Resize handle between results and preview
                let resize_handle = div()
                    .id("resize-handle")
                    .h(px(6.))
                    .w_full()
                    .cursor_row_resize()
                    .bg(cx.theme().colors().border)
                    .hover(|style| style.bg(cx.theme().colors().border_focused))
                    .on_drag(
                        ResizeDrag {
                            mouse_start_y: window.mouse_position().y,
                            results_height_start: self.results_height,
                            preview_height_start: self.preview_height,
                        },
                        |_, _, _, cx| cx.new(|_| DragPreview),
                    )
                    .on_drag_move::<ResizeDrag>(cx.listener(|this, event: &DragMoveEvent<ResizeDrag>, _window, cx| {
                        let drag = event.drag(cx);
                        let delta = event.event.position.y - drag.mouse_start_y;
                        let total_height = drag.results_height_start + drag.preview_height_start;

                        let new_results = (drag.results_height_start + delta)
                            .max(px(MIN_PANEL_HEIGHT))
                            .min(total_height - px(MIN_PANEL_HEIGHT));
                        let new_preview = total_height - new_results;

                        this.results_height = new_results;
                        this.preview_height = new_preview;
                        cx.notify();
                    }));

                // Bottom resize handle for preview
                let bottom_resize_handle = div()
                    .id("bottom-resize-handle")
                    .h(px(6.))
                    .w_full()
                    .cursor_row_resize()
                    .bg(cx.theme().colors().border)
                    .hover(|style| style.bg(cx.theme().colors().border_focused))
                    .on_drag(
                        BottomResizeDrag {
                            mouse_start_y: window.mouse_position().y,
                            preview_height_start: self.preview_height,
                        },
                        |_, _, _, cx| cx.new(|_| DragPreview),
                    )
                    .on_drag_move::<BottomResizeDrag>(cx.listener(
                        |this, event: &DragMoveEvent<BottomResizeDrag>, _window, cx| {
                            let drag = event.drag(cx);
                            let delta = event.event.position.y - drag.mouse_start_y;

                            let new_preview = (drag.preview_height_start + delta)
                                .max(px(MIN_PANEL_HEIGHT))
                                .min(px(MAX_PREVIEW_HEIGHT));

                            this.preview_height = new_preview;
                            cx.notify();
                        },
                    ));

                this.child(resize_handle)
                    .child(
                        v_flex().children(preview_header).child(
                            div()
                                .h(self.preview_height)
                                .overflow_hidden()
                                .child(self.preview_editor.clone()),
                        ),
                    )
                    .child(bottom_resize_handle)
            })
    }
}

pub struct DocsSearchDelegate {
    project: Entity<Project>,
    preview_editor: Entity<Editor>,
    matches: Vec<SearchMatch>,
    selected_index: usize,
    cancel_flag: Arc<std::sync::atomic::AtomicBool>,
    last_selection_change_time: Option<std::time::Instant>,
    last_confirm_time: Option<std::time::Instant>,
    search_options: SearchOptions,
    search_in_progress: bool,
    pending_initial_query: RefCell<Option<String>>,
    panels_with_errors: HashMap<InputPanel, String>,
}

impl DocsSearchDelegate {
    fn new(project: Entity<Project>, preview_editor: Entity<Editor>, initial_query: Option<String>, cx: &App) -> Self {
        Self {
            project,
            preview_editor,
            matches: Vec::new(),
            selected_index: 0,
            cancel_flag: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            last_selection_change_time: None,
            last_confirm_time: None,
            search_options: SearchOptions::from_settings(&EditorSettings::get_global(cx).search),
            search_in_progress: false,
            pending_initial_query: RefCell::new(initial_query),
            panels_with_errors: HashMap::default(),
        }
    }

    fn update_preview(&self, window: &mut Window, cx: &mut Context<Picker<Self>>) {
        let Some(selected_match) = self.matches.get(self.selected_index) else {
            self.preview_editor.update(cx, |editor, cx| {
                editor.buffer().update(cx, |multi_buffer, cx| {
                    multi_buffer.clear(cx);
                });
            });
            return;
        };

        let buffer = selected_match.buffer.clone();
        let range = selected_match.range.clone();
        let anchor_range = selected_match.anchor_range.clone();

        self.preview_editor.update(cx, |editor, cx| {
            let multi_buffer = editor.buffer().clone();
            let buffer_snapshot = buffer.read(cx);
            let max_point = buffer_snapshot.max_point();

            let context_start = buffer_snapshot.anchor_before(Point::new(0, 0));
            let context_end = buffer_snapshot.anchor_after(max_point);

            let primary_range = {
                let start = buffer_snapshot.anchor_before(range.start);
                let end = buffer_snapshot.anchor_after(range.end);
                start..end
            };

            multi_buffer.update(cx, |multi_buffer, cx| {
                multi_buffer.clear(cx);
                multi_buffer.push_excerpts(
                    buffer.clone(),
                    [ExcerptRange {
                        context: context_start..context_end,
                        primary: primary_range,
                    }],
                    cx,
                );
            });

            let multi_buffer_snapshot = multi_buffer.read(cx);
            if let Some(excerpt_id) = multi_buffer_snapshot.excerpt_ids().first().copied() {
                // Highlight the entire row (including gutter)
                let row_anchor = editor::Anchor::in_buffer(excerpt_id, anchor_range.start);
                editor.highlight_rows::<SearchMatchLineHighlight>(
                    row_anchor..row_anchor,
                    cx.theme().colors().editor_active_line_background,
                    RowHighlightOptions::default(),
                    cx,
                );

                // Highlight the match itself
                let highlight_range = editor::Anchor::range_in_buffer(excerpt_id, anchor_range.clone());

                editor.highlight_background::<SearchMatchHighlight>(
                    &[highlight_range],
                    |_, theme| theme.colors().search_match_background,
                    cx,
                );
            }

            let start = multi_buffer::MultiBufferOffset(range.start);
            let end = multi_buffer::MultiBufferOffset(range.end);
            editor.change_selections(Default::default(), window, cx, |s| {
                s.select_ranges([start..end]);
            });
        });
    }

    fn build_search_query(&mut self, query: &str, cx: &mut Context<Picker<Self>>) -> Option<SearchQuery> {
        if query.is_empty() {
            self.panels_with_errors.remove(&InputPanel::Query);
            return None;
        }

        let files_to_include = {
            self.panels_with_errors.remove(&InputPanel::Include);
            PathMatcher::default()
        };

        let files_to_exclude = {
            self.panels_with_errors.remove(&InputPanel::Exclude);
            PathMatcher::default()
        };

        let match_full_paths = self.project.read(cx).visible_worktrees(cx).count() > 1;
        let case_sensitive = self.search_options.contains(SearchOptions::CASE_SENSITIVE);
        let whole_word = self.search_options.contains(SearchOptions::WHOLE_WORD);
        let include_ignored = self.search_options.contains(SearchOptions::INCLUDE_IGNORED);

        let result = if self.search_options.contains(SearchOptions::REGEX) {
            SearchQuery::regex(
                query,
                whole_word,
                case_sensitive,
                include_ignored,
                false,
                files_to_include,
                files_to_exclude,
                match_full_paths,
                None,
            )
        } else {
            SearchQuery::text(
                query,
                whole_word,
                case_sensitive,
                include_ignored,
                files_to_include,
                files_to_exclude,
                match_full_paths,
                None,
            )
        };

        match result {
            Ok(search_query) => {
                if self.panels_with_errors.remove(&InputPanel::Query).is_some() {
                    cx.notify();
                }
                Some(search_query)
            }
            Err(e) => {
                if self
                    .panels_with_errors
                    .insert(InputPanel::Query, e.to_string())
                    .is_none()
                {
                    cx.notify();
                }
                None
            }
        }
    }

    fn process_search_result(
        buffer: &Entity<Buffer>,
        ranges: &[Range<Anchor>],
        cx: &AsyncApp,
    ) -> anyhow::Result<Vec<SearchMatch>> {
        if ranges.is_empty() {
            return Ok(Vec::new());
        }

        let buffer_data = buffer.read_with(cx, |buf, cx| {
            let file = buf.file();
            let path = file.map(|f| ProjectPath {
                worktree_id: f.worktree_id(cx),
                path: f.path().clone(),
            });
            let text = buf.text();

            let mut result = Vec::new();
            for anchor_range in ranges {
                let start_offset: usize = buf.summary_for_anchor(&anchor_range.start);
                let end_offset: usize = buf.summary_for_anchor(&anchor_range.end);
                let match_row = buf.offset_to_point(start_offset).row;
                let line_number = match_row + 1;
                let line_start = text[..start_offset].rfind('\n').map(|i| i + 1).unwrap_or(0);
                let line_end = text[start_offset..]
                    .find('\n')
                    .map(|i| start_offset + i)
                    .unwrap_or(text.len());
                let line_text = text[line_start..line_end].to_string();

                let relative_start = start_offset - line_start;
                let relative_end = end_offset - line_start;

                if let Some(path) = &path {
                    result.push(SearchMatch {
                        path: path.clone(),
                        buffer: buffer.clone(),
                        anchor_range: anchor_range.clone(),
                        range: start_offset..end_offset,
                        relative_range: relative_start..relative_end,
                        line_text,
                        line_number,
                    });
                }
            }
            result
        });

        buffer_data
    }
}

impl PickerDelegate for DocsSearchDelegate {
    type ListItem = ListItem;

    fn placeholder_text(&self, _window: &mut Window, _cx: &mut App) -> Arc<str> {
        "Search documentation...".into()
    }

    fn render_editor(&self, editor: &Entity<Editor>, window: &mut Window, cx: &mut Context<Picker<Self>>) -> Div {
        let search_options = self.search_options;
        let focus_handle = editor.focus_handle(cx);

        let pending_query = self.pending_initial_query.borrow_mut().take();

        editor.update(cx, |editor, cx| {
            if let EditorMode::SingleLine = editor.mode() {
                editor.set_mode(EditorMode::AutoHeight {
                    min_lines: 1,
                    max_lines: Some(4),
                });
                editor.set_soft_wrap_mode(SoftWrap::EditorWidth, cx);
            }
            // Set pending query after mode is changed (so newlines display correctly)
            if let Some(query) = pending_query {
                editor.set_text(query, window, cx);
            }
        });

        v_flex()
            .child(
                h_flex()
                    .flex_none()
                    .min_h_9()
                    .px_2p5()
                    .gap_1()
                    .items_start()
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .py_1p5()
                            .border_1()
                            .rounded_md()
                            .px_1()
                            .border_color(if self.panels_with_errors.contains_key(&InputPanel::Query) {
                                Color::Error.color(cx)
                            } else {
                                gpui::transparent_black()
                            })
                            .child(editor.clone()),
                    )
                    .child({
                        h_flex()
                            .flex_none()
                            .gap_0p5()
                            .pt_1()
                            .child({
                                let editor_for_click = editor.clone();
                                IconButton::new("insert-newline", IconName::Return)
                                    .size(ButtonSize::Compact)
                                    .tooltip(Tooltip::text("Insert New Line"))
                                    .on_click(move |_, window, cx| {
                                        editor_for_click.update(cx, |editor, cx| {
                                            editor.insert("\n", window, cx);
                                        });
                                    })
                            })
                            .child({
                                let focus_handle = focus_handle.clone();
                                IconButton::new("case-sensitive", SearchOption::CaseSensitive.icon())
                                    .size(ButtonSize::Compact)
                                    .toggle_state(search_options.contains(SearchOptions::CASE_SENSITIVE))
                                    .tooltip(move |_window, cx| {
                                        Tooltip::for_action_in(
                                            SearchOption::CaseSensitive.label(),
                                            &ToggleCaseSensitive,
                                            &focus_handle,
                                            cx,
                                        )
                                    })
                                    .on_click(cx.listener(|picker, _, window, cx| {
                                        picker.delegate.search_options.toggle(SearchOptions::CASE_SENSITIVE);
                                        picker.refresh(window, cx);
                                    }))
                            })
                            .child({
                                let focus_handle = focus_handle.clone();
                                IconButton::new("whole-word", SearchOption::WholeWord.icon())
                                    .size(ButtonSize::Compact)
                                    .toggle_state(search_options.contains(SearchOptions::WHOLE_WORD))
                                    .tooltip(move |_window, cx| {
                                        Tooltip::for_action_in(
                                            SearchOption::WholeWord.label(),
                                            &ToggleWholeWord,
                                            &focus_handle,
                                            cx,
                                        )
                                    })
                                    .on_click(cx.listener(|picker, _, window, cx| {
                                        picker.delegate.search_options.toggle(SearchOptions::WHOLE_WORD);
                                        picker.refresh(window, cx);
                                    }))
                            })
                            .child(
                                IconButton::new("regex", SearchOption::Regex.icon())
                                    .size(ButtonSize::Compact)
                                    .toggle_state(search_options.contains(SearchOptions::REGEX))
                                    .tooltip(move |_window, cx| {
                                        Tooltip::for_action_in(
                                            SearchOption::Regex.label(),
                                            &ToggleRegex,
                                            &focus_handle,
                                            cx,
                                        )
                                    })
                                    .on_click(cx.listener(|picker, _, window, cx| {
                                        picker.delegate.search_options.toggle(SearchOptions::REGEX);
                                        picker.refresh(window, cx);
                                    })),
                            )
                    }),
            )
            .when(self.editor_position() == PickerEditorPosition::Start, |this| {
                this.child(Divider::horizontal())
            })
    }

    fn match_count(&self) -> usize {
        self.matches.len()
    }

    fn selected_index(&self) -> usize {
        self.selected_index
    }

    fn set_selected_index(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Picker<Self>>) {
        self.selected_index = ix;
        self.last_selection_change_time = Some(std::time::Instant::now());
        self.update_preview(window, cx);
    }

    fn update_matches(&mut self, query: String, window: &mut Window, cx: &mut Context<Picker<Self>>) -> Task<()> {
        if !query.is_empty() {
            save_recent_query(&query, cx);
        }

        self.cancel_flag.store(true, std::sync::atomic::Ordering::SeqCst);
        self.cancel_flag = Arc::new(std::sync::atomic::AtomicBool::new(false));

        let cancel_flag = self.cancel_flag.clone();

        let Some(search_query) = self.build_search_query(&query, cx) else {
            self.matches.clear();
            self.selected_index = 0;
            self.search_in_progress = false;
            cx.notify();
            return Task::ready(());
        };

        let (tx, rx) = unbounded();

        let search_task = cx.spawn_in(window, {
            let cancel_flag = cancel_flag.clone();
            async move |_picker, cx| {
                let doc_paths = match Docs.list("") {
                    Ok(paths) => paths,
                    Err(e) => {
                        log::error!("Failed to list documentation files: {}", e);
                        return;
                    }
                };

                for doc_path in doc_paths {
                    if cancel_flag.load(std::sync::atomic::Ordering::SeqCst) {
                        break;
                    }

                    let data = match Docs::get(&doc_path) {
                        Some(data) => data,
                        None => {
                            log::error!("Failed to load {}", doc_path);
                            continue;
                        }
                    };
                    let content = match str::from_utf8(data.data.as_ref()) {
                        Ok(data) => data,
                        Err(e) => {
                            log::error!("Failed to parse {}: {}", doc_path, e);
                            continue;
                        }
                    };

                    let doc_file = DocFile::new(doc_path.to_string());

                    let buffer = cx
                        .new(|cx| {
                            let text_buffer = text::Buffer::new(
                                ReplicaId::LOCAL,
                                BufferId::new(cx.entity_id().as_non_zero_u64().get()).unwrap(),
                                content,
                            );
                            Buffer::build(text_buffer, Some(doc_file), Capability::ReadOnly)
                        })
                        .ok();

                    let Some(buffer) = buffer else {
                        continue;
                    };

                    let snapshot = cx.read_entity(&buffer, |buffer, _cx| buffer.snapshot()).ok();
                    let Some(snapshot) = snapshot else {
                        continue;
                    };

                    let ranges = search_query.search(&snapshot, None).await;

                    if !ranges.is_empty() {
                        let anchor_ranges: Vec<Range<Anchor>> = ranges
                            .iter()
                            .map(|range| {
                                let start = snapshot.anchor_before(range.start);
                                let end = snapshot.anchor_after(range.end);
                                start..end
                            })
                            .collect();

                        if tx
                            .send(SearchResult::Buffer {
                                buffer: buffer.clone(),
                                ranges: anchor_ranges,
                            })
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
        });

        let search_results = SearchResults {
            rx,
            _task_handle: search_task,
        };

        self.matches.clear();
        self.selected_index = 0;
        self.search_in_progress = true;
        cx.notify();

        cx.spawn_in(window, async move |picker, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(SEARCH_DEBOUNCE_MS))
                .await;

            if cancel_flag.load(std::sync::atomic::Ordering::SeqCst) {
                return;
            }

            let SearchResults { rx, _task_handle } = search_results;
            let mut results_stream = pin!(rx.ready_chunks(256));

            while let Some(results) = results_stream.next().await {
                if cancel_flag.load(std::sync::atomic::Ordering::SeqCst) {
                    break;
                }

                let mut batch_matches = Vec::new();
                let mut limit_reached = false;

                for result in results {
                    match result {
                        SearchResult::Buffer { buffer, ranges } => {
                            if let Ok(matches) = DocsSearchDelegate::process_search_result(&buffer, &ranges, cx) {
                                batch_matches.extend(matches);
                            }
                        }
                        SearchResult::LimitReached => {
                            limit_reached = true;
                        }
                    }
                }

                picker
                    .update_in(cx, |picker, window, cx| {
                        let delegate = &mut picker.delegate;
                        delegate.matches.extend(batch_matches);

                        if delegate.selected_index >= delegate.matches.len() && !delegate.matches.is_empty() {
                            delegate.selected_index = 0;
                        }

                        if delegate.matches.len() == delegate.selected_index + 1 || delegate.selected_index == 0 {
                            delegate.update_preview(window, cx);
                        }

                        cx.notify();
                    })
                    .log_err();

                if limit_reached {
                    break;
                }

                smol::future::yield_now().await;
            }

            picker
                .update_in(cx, |picker, _window, cx| {
                    picker.delegate.search_in_progress = false;
                    cx.notify();
                })
                .log_err();
        })
    }

    fn confirm(&mut self, _secondary: bool, window: &mut Window, cx: &mut Context<Picker<Self>>) {
        // Clicks (set_selected_index called immediately before confirm) require double-click.
        // Enter key proceeds immediately.
        let now = std::time::Instant::now();
        let is_click = self
            .last_selection_change_time
            .map(|t| now.duration_since(t).as_millis() < 50)
            .unwrap_or(false);

        if is_click {
            let is_double_click = self
                .last_confirm_time
                .map(|t| now.duration_since(t).as_millis() < 300)
                .unwrap_or(false);
            self.last_confirm_time = Some(now);

            if !is_double_click {
                return;
            }
        }

        let Some(selected_match) = self.matches.get(self.selected_index) else {
            return;
        };

        let path = selected_match.path.path.clone().display(PathStyle::local()).to_string();

        window.dispatch_action(Box::new(app_actions::OpenDocsAt { path }), cx);

        cx.emit(DismissEvent);
    }

    fn dismissed(&mut self, _window: &mut Window, _cx: &mut Context<Picker<Self>>) {}

    fn render_match(
        &self,
        ix: usize,
        selected: bool,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Option<Self::ListItem> {
        let search_match = self.matches.get(ix)?;
        let path_style = self.project.read(cx).path_style(cx);
        let path_str = search_match.path.path.display(path_style).to_string();

        let original_line = &search_match.line_text;
        let line_text = original_line.trim_start();
        let trim_offset = original_line.len() - line_text.len();
        let line_text_string = line_text.to_string();

        // Build search match range (takes precedence over syntax highlighting)
        let search_match_style = HighlightStyle {
            background_color: Some(cx.theme().colors().search_match_background),
            font_weight: Some(gpui::FontWeight::BOLD),
            ..Default::default()
        };

        let mut highlights: Vec<(Range<usize>, HighlightStyle)> = Vec::new();

        // Get syntax highlighting from the buffer
        {
            let line_start_abs = search_match.range.start - search_match.relative_range.start;
            let visible_start_abs = line_start_abs + trim_offset;
            let visible_end_abs = line_start_abs + original_line.len();
            let match_start_abs = search_match.range.start;
            let match_end_abs = search_match.range.end;

            // Determine the "effective" match range within the visible area
            let effective_match_start = match_start_abs.max(visible_start_abs);
            let effective_match_end = match_end_abs.min(visible_end_abs);

            let ranges = [
                (visible_start_abs..effective_match_start, false),
                (effective_match_start..effective_match_end, true),
                (effective_match_end..visible_end_abs, false),
            ];

            let snapshot = search_match.buffer.read(cx).snapshot();
            let syntax_theme = cx.theme().syntax().clone();
            let mut current_offset = 0;

            for (range, is_match) in ranges {
                if range.start >= range.end {
                    continue;
                }

                for chunk in snapshot.chunks(range, true) {
                    let chunk_len = chunk.text.len();
                    let syntax_style = chunk.syntax_highlight_id.and_then(|id| id.style(&syntax_theme));

                    let style = if is_match {
                        let mut style = syntax_style.unwrap_or_default();
                        if let Some(bg) = search_match_style.background_color {
                            style.background_color = Some(bg);
                        }
                        if let Some(weight) = search_match_style.font_weight {
                            style.font_weight = Some(weight);
                        }
                        style
                    } else {
                        syntax_style.unwrap_or_default()
                    };

                    highlights.push((current_offset..current_offset + chunk_len, style));
                    current_offset += chunk_len;
                }
            }
        }

        let text_style = window.text_style();

        Some(
            ListItem::new(ix)
                .inset(true)
                .spacing(ListItemSpacing::Sparse)
                .toggle_state(selected)
                .child(
                    h_flex()
                        .w_full()
                        .gap_4()
                        .justify_between()
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .overflow_hidden()
                                .text_ellipsis()
                                .whitespace_nowrap()
                                .child(
                                    StyledText::new(line_text_string).with_default_highlights(&text_style, highlights),
                                ),
                        )
                        .child(
                            h_flex()
                                .w(relative(0.35))
                                .flex_none()
                                .justify_between()
                                .gap_2()
                                .child(
                                    div().flex_1().overflow_hidden().child(
                                        Label::new(path_str)
                                            .size(LabelSize::Small)
                                            .color(Color::Muted)
                                            .truncate(),
                                    ),
                                )
                                .child(
                                    div().pr_2().child(
                                        Label::new(search_match.line_number.to_string())
                                            .size(LabelSize::Small)
                                            .color(Color::Muted),
                                    ),
                                ),
                        ),
                ),
        )
    }
}
