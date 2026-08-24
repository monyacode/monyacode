use std::cmp::min;
use std::path::{Path, PathBuf};
use std::{borrow::Cow, sync::Arc, time::Duration};

use anyhow::Result;
use app_actions::{ChangeKeybinding, OpenDocs, OpenDocsAt, OpenMonyaCodeUrl, docs_search};
use assets::lookup_docs;
use editor::EditorEvent;
use gpui::{
    Action, Entity, EventEmitter, FocusHandle, Focusable, IsZero as _, ListState, RetainAllImageCache, Task,
    WeakEntity, list,
};
use language::LanguageRegistry;
use markdown_preview::markdown_elements::{Link, ParsedMarkdown, ParsedMarkdownElement};
use markdown_preview::markdown_parser::parse_markdown;
use markdown_preview::markdown_renderer::{MermaidState, RenderContext, render_markdown_block};
use markdown_preview::{ScrollDown, ScrollDownByItem, ScrollPageDown, ScrollPageUp, ScrollUp, ScrollUpByItem};
use regex::Regex;
use settings::Settings as _;
use theme::ThemeSettings;
use ui::{Tooltip, WithScrollbar as _, prelude::*};
use workspace::{Item, ItemHandle, Workspace};

const REPARSE_DEBOUNCE: Duration = Duration::from_millis(200);

pub fn init(cx: &mut App) {
    cx.observe_new(DocumentationView::register).detach();
}

pub(crate) struct DocumentationView {
    workspace: WeakEntity<Workspace>,
    image_cache: Entity<RetainAllImageCache>,
    focus_handle: FocusHandle,
    language_registry: Arc<LanguageRegistry>,
    contents: Option<ParsedMarkdown>,
    selected_block: usize,
    list_state: ListState,
    current: SharedString,
    back: Vec<SharedString>,
    forward: Vec<SharedString>,
    mermaid_state: MermaidState,
    parsing_markdown_task: Option<Task<Result<()>>>,
}

impl EventEmitter<EditorEvent> for DocumentationView {}

impl Render for DocumentationView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self.current.clone();
        let settings = ThemeSettings::get_global(cx);
        let buffer_size = settings.buffer_font_size(cx);
        let line_height = (2 * buffer_size).round();
        let theme = cx.theme();

        v_flex()
            .image_cache(self.image_cache.clone())
            .id("DocumentationView")
            .key_context("MarkdownPreview")
            .track_focus(&self.focus_handle(cx))
            .on_action(cx.listener(DocumentationView::scroll_page_up))
            .on_action(cx.listener(DocumentationView::scroll_page_down))
            .on_action(cx.listener(DocumentationView::scroll_up))
            .on_action(cx.listener(DocumentationView::scroll_down))
            .on_action(cx.listener(DocumentationView::scroll_up_by_item))
            .on_action(cx.listener(DocumentationView::scroll_down_by_item))
            .text_size(buffer_size)
            .line_height(line_height)
            .size_full()
            .p_4()
            .gap_2()
            .bg(theme.colors().editor_background)
            .child(self.render_toolbar(window, cx))
            .child(div().flex_grow().map(|this| {
                let current = current.clone();
                this.child(
                    list(
                        self.list_state.clone(),
                        cx.processor(move |this, ix, window, cx| {
                            this.render_markdown_block(current.clone(), ix, window, cx)
                        }),
                    )
                    .size_full(),
                )
            }))
            .vertical_scrollbar_for(&self.list_state, window, cx)
    }
}

pub fn open_search(window: &mut Window, cx: &mut App) {
    window.dispatch_action(Box::new(docs_search::Toggle), cx);
}

pub fn open_doc_url(url: SharedString, window: &mut Window, cx: &mut App) {
    if url.starts_with("monyacode://docs/") {
        window.dispatch_action(Box::new(OpenDocsAt { path: url.into() }), cx);
    } else if url.starts_with("monyacode://kb/") {
        let action = url.strip_prefix("monyacode://kb/").unwrap().to_string();
        window.dispatch_action(Box::new(ChangeKeybinding { action }), cx);
    } else if url.starts_with("monyacode://action/") {
        let action = url.strip_prefix("monyacode://action/").unwrap().to_string();
        window.dispatch_action(Box::new(ChangeKeybinding { action }), cx);
    } else if url.starts_with("monyacode://") {
        window.dispatch_action(Box::new(OpenMonyaCodeUrl { url: url.into() }), cx);
    } else if url.starts_with("http://") || url.starts_with("https://") {
        cx.open_url(&url);
    } else {
        let url = url.strip_prefix("./").unwrap_or(&url);
        // TODO: Anchor links are not supported yet
        if url.starts_with("#") {
            return;
        }
        let url = url.split_once("#").map(|s| s.0).unwrap_or(&url);
        let url = format!("monyacode://docs/{url}");
        let url = if url.contains(".md#") {
            url.replace(".md#", "#")
        } else if !url.ends_with(".md") {
            url + ".md"
        } else {
            url
        };
        window.dispatch_action(Box::new(OpenDocsAt { path: url }), cx);
    }
}

impl DocumentationView {
    fn render_toolbar(&self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_2().child(
            h_flex()
                .gap_4()
                .child(
                    IconButton::new("doc-view-toc", IconName::Library)
                        .tooltip(Tooltip::text("Table of Contents"))
                        .on_click(move |_, window, cx| {
                            open_doc_url("monyacode://docs/index.md".into(), window, cx);
                        }),
                )
                .child(
                    IconButton::new("doc-view-search", IconName::MagnifyingGlass)
                        .tooltip(Tooltip::text("Search Documentation"))
                        .on_click(move |_, window, cx| {
                            open_search(window, cx);
                        }),
                )
                .child(
                    IconButton::new("doc-view-back", IconName::ArrowLeft)
                        .disabled(self.back.is_empty())
                        .tooltip(Tooltip::text("Back"))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.go_back(window, cx);
                        })),
                )
                .child(
                    IconButton::new("doc-view-forward", IconName::ArrowRight)
                        .disabled(self.forward.is_empty())
                        .tooltip(Tooltip::text("Forward"))
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.go_forward(window, cx);
                        })),
                ),
        )
    }

    fn render_markdown_block(
        &mut self,
        current: SharedString,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(contents) = &self.contents else {
            return div().into_any();
        };

        let mut render_cx = RenderContext::new(Some(self.workspace.clone()), &self.mermaid_state, window, cx)
            .with_link_clicked_callback(move |link: Link, window, cx| match link {
                Link::Web { url } => open_doc_url(url.into(), window, cx),
                Link::Path { path, .. } => {
                    let from = if let Some(base) = Path::new(current.as_str()).parent() {
                        let path = path.to_str().unwrap().to_string();
                        if path.starts_with("../") {
                            Path::new(".").join(base.parent().unwrap().join(path.strip_prefix("../").unwrap()))
                        } else if path.starts_with("./") {
                            base.join(path.strip_prefix("./").unwrap())
                        } else {
                            base.join(path.as_str())
                        }
                    } else {
                        path
                    };

                    open_doc_url(SharedString::from(from.to_str().unwrap().to_string()), window, cx)
                }
            });

        let block = contents.children.get(ix).unwrap();
        let rendered_block = render_markdown_block(block, &mut render_cx);
        let selected_block = self.selected_block;
        let scaled_rems = render_cx.scaled_rems(1.0);

        let should_apply_padding = Self::should_apply_padding_between(block, contents.children.get(ix + 1));

        div()
            .id(ix)
            .when(should_apply_padding, |this| this.pb(render_cx.scaled_rems(0.75)))
            .group("markdown-block")
            .map(move |container| {
                let indicator = div()
                    .h_full()
                    .w(px(4.0))
                    .when(ix == selected_block, |this| this.bg(cx.theme().colors().border))
                    .group_hover("markdown-block", |s| {
                        if ix == selected_block {
                            s
                        } else {
                            s.bg(cx.theme().colors().border_variant)
                        }
                    })
                    .rounded_xs();

                container.child(
                    div()
                        .relative()
                        .child(div().pl(scaled_rems).child(rendered_block))
                        .child(indicator.absolute().left_0().top_0()),
                )
            })
            .into_any()
    }

    pub fn register(workspace: &mut Workspace, _window: Option<&mut Window>, _cx: &mut Context<Workspace>) {
        workspace
            .register_action(move |workspace, _: &OpenDocs, window, cx| {
                DocumentationView::open_documentation_page(workspace, None, window, cx);
                cx.notify();
            })
            .register_action(move |workspace, OpenDocsAt { path }: &OpenDocsAt, window, cx| {
                DocumentationView::open_documentation_page(workspace, Some(path.into()), window, cx);
                cx.notify();
            });
    }

    fn new(
        path: &str,
        workspace: WeakEntity<Workspace>,
        window: &mut Window,
        language_registry: Arc<LanguageRegistry>,
        cx: &mut Context<Self>,
    ) -> Self {
        let list_state = ListState::new(0, gpui::ListAlignment::Top, px(1000.));
        let focus_handle = cx.focus_handle();
        cx.on_focus_in(&focus_handle, window, Self::focus_in).detach();

        let mut this = Self {
            workspace,
            selected_block: 0,
            focus_handle,
            language_registry,
            current: SharedString::from(""),
            back: Vec::new(),
            forward: Vec::new(),
            mermaid_state: Default::default(),
            parsing_markdown_task: None,
            image_cache: RetainAllImageCache::new(cx),
            contents: None,
            list_state,
        };
        this.set_path(path, window, cx);
        this
    }

    fn set_path(&mut self, path: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.current == path {
            return;
        }

        self.current = SharedString::from(path.to_string());

        self.parse_markdown_from_text(false, window, cx);
    }

    fn parse_markdown_from_text(&mut self, wait_for_debounce: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = get_docs(&self.current) {
            self.parsing_markdown_task =
                Some(self.parse_markdown_in_background(wait_for_debounce, text.into(), window, cx));
        }
    }

    fn parse_markdown_in_background(
        &mut self,
        wait_for_debounce: bool,
        text: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Task<Result<()>> {
        let language_registry = self.language_registry.clone();
        let actions = window.available_actions(cx);

        cx.spawn_in(window, async move |view, cx| {
            if wait_for_debounce {
                // Wait for the user to stop typing
                cx.background_executor().timer(REPARSE_DEBOUNCE).await;
            }

            let parsing_task = cx.background_spawn(async move {
                let text: SharedString = if text.contains("{#") {
                    preprocess_text(&text, actions).into()
                } else {
                    text
                };
                parse_markdown(&text, Some(PathBuf::new()), Some(language_registry)).await
            });
            let contents = parsing_task.await;
            view.update(cx, move |view, cx| {
                let markdown_blocks_count = contents.children.len();
                view.contents = Some(contents);
                let scroll_top = view.list_state.logical_scroll_top();
                view.list_state.reset(markdown_blocks_count);
                view.list_state.scroll_to(scroll_top);
                cx.notify();
            })
        })
    }

    fn should_apply_padding_between(
        current_block: &ParsedMarkdownElement,
        next_block: Option<&ParsedMarkdownElement>,
    ) -> bool {
        !(current_block.is_list_item() && next_block.map(|b| b.is_list_item()).unwrap_or(false))
    }

    fn scroll_page_up(&mut self, _: &ScrollPageUp, _window: &mut Window, cx: &mut Context<Self>) {
        let viewport_height = self.list_state.viewport_bounds().size.height;
        if viewport_height.is_zero() {
            return;
        }

        self.list_state.scroll_by(-viewport_height);
        cx.notify();
    }

    fn scroll_page_down(&mut self, _: &ScrollPageDown, _window: &mut Window, cx: &mut Context<Self>) {
        let viewport_height = self.list_state.viewport_bounds().size.height;
        if viewport_height.is_zero() {
            return;
        }

        self.list_state.scroll_by(viewport_height);
        cx.notify();
    }

    fn scroll_up(&mut self, _: &ScrollUp, window: &mut Window, cx: &mut Context<Self>) {
        let scroll_top = self.list_state.logical_scroll_top();
        if let Some(bounds) = self.list_state.bounds_for_item(scroll_top.item_ix) {
            let item_height = bounds.size.height;
            // Scroll no more than the rough equivalent of a large headline
            let max_height = window.rem_size() * 2;
            let scroll_height = min(item_height, max_height);
            self.list_state.scroll_by(-scroll_height);
        }
        cx.notify();
    }

    fn scroll_down(&mut self, _: &ScrollDown, window: &mut Window, cx: &mut Context<Self>) {
        let scroll_top = self.list_state.logical_scroll_top();
        if let Some(bounds) = self.list_state.bounds_for_item(scroll_top.item_ix) {
            let item_height = bounds.size.height;
            // Scroll no more than the rough equivalent of a large headline
            let max_height = window.rem_size() * 2;
            let scroll_height = min(item_height, max_height);
            self.list_state.scroll_by(scroll_height);
        }
        cx.notify();
    }

    fn scroll_up_by_item(&mut self, _: &ScrollUpByItem, _window: &mut Window, cx: &mut Context<Self>) {
        let scroll_top = self.list_state.logical_scroll_top();
        if let Some(bounds) = self.list_state.bounds_for_item(scroll_top.item_ix) {
            self.list_state.scroll_by(-bounds.size.height);
        }
        cx.notify();
    }

    fn scroll_down_by_item(&mut self, _: &ScrollDownByItem, _window: &mut Window, cx: &mut Context<Self>) {
        let scroll_top = self.list_state.logical_scroll_top();
        if let Some(bounds) = self.list_state.bounds_for_item(scroll_top.item_ix) {
            self.list_state.scroll_by(bounds.size.height);
        }
        cx.notify();
    }

    fn focus_in(&mut self, window: &mut Window, _cx: &mut Context<Self>) {
        if self.focus_handle.is_focused(window) {}
    }

    pub(crate) fn open_documentation_page(
        workspace: &mut Workspace,
        path: Option<String>,
        window: &mut Window,
        cx: &mut Context<'_, Workspace>,
    ) {
        let language_registry = workspace.project().read(cx).languages().clone();
        let workspace_handle = workspace.weak_handle();
        let path = path.unwrap_or("index.md".into());
        let path = path.strip_prefix("monyacode://docs/").unwrap_or(&path);
        if let Some(existing) = workspace.item_of_type::<DocumentationView>(cx) {
            let is_active = workspace
                .active_item(cx)
                .is_some_and(|item| item.item_id() == existing.item_id());

            existing.update(cx, |this, cx| this.update_text(path, false, window, cx));
            workspace.activate_item(&existing, true, !is_active, window, cx);
        } else {
            let view = cx.new(|cx| DocumentationView::new(path, workspace_handle, window, language_registry, cx));
            workspace.add_item_to_active_pane(Box::new(view), None, true, window, cx);
        }
    }

    pub(crate) fn update_text(&mut self, path: &str, nav: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.current != path {
            let current = self.current.clone();
            self.back.push(current);
            if !nav {
                self.forward.clear();
            }
            self.set_path(path, window, cx);
            cx.notify();
        }
    }

    pub(crate) fn go_back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = self.back.pop() {
            let current = self.current.clone();
            self.forward.push(current);
            self.set_path(&path, window, cx);
        }
    }

    pub(crate) fn go_forward(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(path) = self.forward.pop() {
            self.set_path(&path, window, cx);
        }
    }
}

fn preprocess_text(text: &str, actions: Vec<Box<dyn Action>>) -> String {
    const TABLE_KEY: &str = "{#ACTIONS_TABLE#}";
    let text = if text.contains(TABLE_KEY) {
        let mut sorted: Vec<String> = actions
            .into_iter()
            .map(|a| format!("- [{}](monyacode://action/{})", a.name(), a.name()))
            .collect::<Vec<_>>();
        sorted.sort();
        let table_re = Regex::new(r"\{#ACTIONS_TABLE#\}").unwrap();
        table_re.replace_all(text, &sorted.join("\n"))
    } else {
        text.into()
    };
    let kb_re = Regex::new(r"\{\#kb\s+(\w+:+\w+)\s*\}").unwrap();
    let action_re = Regex::new(r"\{\#action\s+(\w+:+\w+)\s*\}").unwrap();
    let text = kb_re.replace_all(&text, "[${1}](monyacode://kb/${1})");
    let text = action_re.replace_all(&text, "[${1}](monyacode://action/${1})");
    text.to_string()
}

fn get_docs<'b>(path: &str) -> Option<Cow<'b, str>> {
    if let Some(text) = lookup_docs(&path) {
        Some(match text.data {
            Cow::Borrowed(bytes) => Cow::Borrowed(std::str::from_utf8(bytes).unwrap()),
            Cow::Owned(bytes) => Cow::Owned(String::from_utf8(bytes).unwrap()),
        })
    } else {
        None
    }
}

impl Focusable for DocumentationView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Item for DocumentationView {
    type Event = EditorEvent;

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        SharedString::from(self.current.strip_suffix(".md").unwrap_or(&self.current).to_string())
    }
}
