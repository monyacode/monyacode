use std::{collections::HashMap, sync::Arc};

use editor::EditorEvent;
use gpui::{EventEmitter, FocusHandle, Focusable, Global, ScrollHandle, actions};
use language::{BinaryStatus, LanguageRegistry};
use lsp::LanguageServerName;
use project::{
    Fs, LspStoreEvent,
    project_settings::{NodeBinarySettings, ProjectSettings},
};
use proto::{
    self, ServerBinaryStatus,
    status_update::Status,
    update_language_server::Variant::{RegisteredForBuffer, StatusUpdate},
};
use settings::{BinarySettings, Settings, SettingsStore};
use theme::Theme;
use ui::{
    ActiveTheme as _, App, Context, IntoElement, Label, LabelSize, Render, Switch, ToggleState, Tooltip, Window,
    WithScrollbar as _, prelude::*,
};
use workspace::{Item, ItemHandle as _, Workspace};

actions!(
    lsp,
    [
        /// Opens the language server configuration view.
        OpenLanguageServerConfig
    ]
);

struct LspConfigState {
    binary_statuses: HashMap<LanguageServerName, (BinaryStatus, Option<SharedString>)>,
}

impl Global for LspConfigState {}

pub fn init(cx: &mut App) {
    cx.set_global(LspConfigState {
        binary_statuses: HashMap::new(),
    });

    cx.observe_new(LspConfigView::register).detach();
    cx.observe_new(|workspace: &mut Workspace, window, cx| {
        if let Some(window) = window {
            LspConfigView::init_workspace_tracking(workspace, window, cx);
        }
    })
    .detach();
}

pub(crate) struct LspConfigView {
    _language_registry: Arc<LanguageRegistry>,
    focus_handle: FocusHandle,
    scroll_handle: ScrollHandle,
    _binary_status_task: Option<gpui::Task<()>>,
}

impl LspConfigView {
    pub fn register(workspace: &mut Workspace, _window: Option<&mut Window>, _cx: &mut Context<Workspace>) {
        workspace.register_action(move |workspace, _: &OpenLanguageServerConfig, window, cx| {
            LspConfigView::open(workspace, window, cx);
            cx.notify();
        });
    }

    fn init_workspace_tracking(workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
        let lsp_store = workspace.project().read(cx).lsp_store();

        let server_statuses: Vec<_> = lsp_store
            .read(cx)
            .language_server_statuses()
            .map(|(_, status)| status.name.clone())
            .collect();

        {
            let state = cx.global_mut::<LspConfigState>();
            for name in server_statuses {
                state.binary_statuses.insert(name, (BinaryStatus::None, None));
            }
        }

        cx.subscribe_in(&lsp_store, window, |_workspace, _lsp_store, event, _window, cx| {
            let state = cx.global_mut::<LspConfigState>();
            match event {
                LspStoreEvent::LanguageServerAdded(_server_id, name, _) => {
                    state
                        .binary_statuses
                        .entry(name.clone())
                        .or_insert((BinaryStatus::None, None));
                }
                LspStoreEvent::LanguageServerUpdate {
                    name,
                    message: RegisteredForBuffer(_),
                    ..
                } => {
                    if let Some(name) = name {
                        state
                            .binary_statuses
                            .entry(name.clone())
                            .or_insert((BinaryStatus::None, None));
                    }
                }
                LspStoreEvent::LanguageServerUpdate {
                    name,
                    message: StatusUpdate(status_update),
                    ..
                } => {
                    if let Some(Status::Binary(binary_status_proto)) = &status_update.status {
                        if let Some(name) = name.as_ref() {
                            if let Ok(binary_status) = ServerBinaryStatus::try_from(*binary_status_proto) {
                                let status = match binary_status {
                                    ServerBinaryStatus::None => BinaryStatus::None,
                                    ServerBinaryStatus::CheckingForUpdate => BinaryStatus::CheckingForUpdate,
                                    ServerBinaryStatus::Downloading => BinaryStatus::Downloading,
                                    ServerBinaryStatus::Starting => BinaryStatus::Starting,
                                    ServerBinaryStatus::Stopping => BinaryStatus::Stopping,
                                    ServerBinaryStatus::Stopped => BinaryStatus::Stopped,
                                    ServerBinaryStatus::Failed => {
                                        if let Some(error) = status_update.message.clone() {
                                            BinaryStatus::Failed { error }
                                        } else {
                                            BinaryStatus::Failed {
                                                error: "Unknown error".to_string(),
                                            }
                                        }
                                    }
                                };
                                state.binary_statuses.insert(
                                    name.clone(),
                                    (status, status_update.message.as_ref().map(|s| s.clone().into())),
                                );
                            }
                        }
                    }
                }
                _ => {}
            }
        })
        .detach();
    }

    pub(crate) fn open(workspace: &mut Workspace, window: &mut Window, cx: &mut Context<'_, Workspace>) {
        if let Some(existing) = workspace.item_of_type::<LspConfigView>(cx) {
            let is_active = workspace
                .active_item(cx)
                .is_some_and(|item| item.item_id() == existing.item_id());
            workspace.activate_item(&existing, true, !is_active, window, cx);
        } else {
            let language_registry = workspace.project().read(cx).languages().clone();
            let view = cx.new(|cx| LspConfigView::new(language_registry, cx));
            workspace.add_item_to_active_pane(Box::new(view), None, true, window, cx);
        }
    }

    fn new(language_registry: Arc<LanguageRegistry>, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        let this = Self {
            _language_registry: language_registry,
            focus_handle,
            scroll_handle: ScrollHandle::new(),
            _binary_status_task: None,
        };
        this
    }

    fn toggle_node_setting(
        &mut self,
        f: impl FnOnce(&mut settings::NodeBinarySettings) -> () + std::marker::Send + 'static,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.update_global(|store: &mut SettingsStore, cx| {
            store.update_settings_file(<dyn Fs>::global(cx), move |settings, _cx| {
                let node = settings.node.get_or_insert_with(Default::default);
                f(node);
            });
        });

        window.refresh();
        cx.notify();
    }

    fn toggle_allow_download_node(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.toggle_node_setting(
            |node| {
                node.allow_binary_download = Some(!node.allow_binary_download.unwrap_or_default());
            },
            window,
            cx,
        )
    }

    fn toggle_allow_download_prettier(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.toggle_node_setting(
            |node| {
                node.allow_prettier_download = Some(!node.allow_prettier_download.unwrap_or_default());
            },
            window,
            cx,
        )
    }

    fn toggle_allow_npm_install(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.toggle_node_setting(
            |node| {
                node.allow_npm_install = Some(!node.allow_npm_install.unwrap_or_default());
            },
            window,
            cx,
        )
    }

    fn update_binary_setting<F>(
        &mut self,
        server_name: LanguageServerName,
        updater: F,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) where
        F: FnOnce(&mut BinarySettings) -> () + std::marker::Send + 'static,
    {
        cx.update_global(|store: &mut SettingsStore, cx| {
            store.update_settings_file(<dyn Fs>::global(cx), move |settings, _cx| {
                let lsp_settings = settings
                    .project
                    .lsp
                    .0
                    .entry(Arc::from(server_name.0.as_ref()))
                    .or_insert_with(Default::default);

                let binary = lsp_settings.binary.get_or_insert_with(Default::default);
                updater(binary);
            });
        });

        window.refresh();
        cx.notify();
    }

    fn get_status_info(binary_status: &BinaryStatus) -> (SharedString, Color) {
        match binary_status {
            BinaryStatus::None => ("Running".into(), Color::Success),
            BinaryStatus::Starting => ("Starting".into(), Color::Accent),
            BinaryStatus::Downloading => ("Downloading".into(), Color::Accent),
            BinaryStatus::CheckingForUpdate => ("Checking".into(), Color::Muted),
            BinaryStatus::Stopping => ("Stopping".into(), Color::Warning),
            BinaryStatus::Stopped => ("Stopped".into(), Color::Muted),
            BinaryStatus::Failed { error } => (error.into(), Color::Error),
        }
    }
}

impl EventEmitter<EditorEvent> for LspConfigView {}

impl LspConfigView {
    fn render_server_header(
        &self,
        server_name: SharedString,
        status_text: SharedString,
        status_color: Color,
    ) -> impl IntoElement {
        h_flex().gap_3().items_center().justify_between().child(
            h_flex()
                .gap_1p5()
                .items_center()
                .child(
                    IconButton::new(format!("status-icon-{}", server_name), IconName::Bot)
                        .icon_color(status_color)
                        .tooltip(Tooltip::text(status_text)),
                )
                .child(Label::new(server_name).size(LabelSize::Default)),
        )
    }

    fn render_error_message(msg: &Option<String>) -> impl IntoElement {
        v_flex().when_some(msg.clone(), |this, msg| {
            this.child(
                Label::new(format!("Error: {}", msg))
                    .size(LabelSize::Small)
                    .color(Color::Error)
                    .truncate(),
            )
        })
    }

    fn render_binary_settings(
        &self,
        server_name: &LanguageServerName,
        binary_settings: &settings::BinarySettings,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        v_flex()
            .p_2()
            .gap_4()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Switch::new(
                            format!("allow-binary-download-{}", server_name.0),
                            ToggleState::from(binary_settings.allow_binary_download),
                        )
                        .on_click({
                            let server_name = server_name.clone();
                            cx.listener(move |this, _, window, cx| {
                                this.update_binary_setting(
                                    server_name.clone(),
                                    |binary| {
                                        binary.allow_binary_download =
                                            Some(!binary.allow_binary_download.unwrap_or_default());
                                    },
                                    window,
                                    cx,
                                )
                            })
                        }),
                    )
                    .child(
                        v_flex().gap_1p5().child(Label::new("Allow Download")).child(
                            Label::new("Allow the editor to download a language server binary (if available)")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        ),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Switch::new(
                            format!("ignore-system-{}", server_name.0),
                            ToggleState::from(binary_settings.ignore_system_version),
                        )
                        .on_click({
                            let server_name = server_name.clone();
                            cx.listener(move |this, _, window, cx| {
                                this.update_binary_setting(
                                    server_name.clone(),
                                    |binary| {
                                        binary.ignore_system_version =
                                            Some(!binary.ignore_system_version.unwrap_or_default());
                                    },
                                    window,
                                    cx,
                                )
                            })
                        }),
                    )
                    .child(
                        v_flex().gap_1p5().child(Label::new("Ignore System Version")).child(
                            Label::new("Ignore any language server binary installed system-wide")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        ),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Switch::new(
                            format!("auto-update-{}", server_name.0),
                            ToggleState::from(binary_settings.enable_auto_updates),
                        )
                        .on_click({
                            let server_name = server_name.clone();
                            cx.listener(move |this, _, window, cx| {
                                this.update_binary_setting(
                                    server_name.clone(),
                                    |binary| {
                                        binary.enable_auto_updates =
                                            Some(!binary.enable_auto_updates.unwrap_or_default());
                                    },
                                    window,
                                    cx,
                                )
                            })
                        }),
                    )
                    .child(
                        v_flex().gap_1p5().child(Label::new("Auto Update")).child(
                            Label::new("Automatically download new versions of the language server when released")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        ),
                    ),
            )
    }

    fn render_header(&self) -> impl IntoElement {
        div()
            .flex()
            .items_center()
            .justify_between()
            .pb_2()
            .child(Headline::new("Language Server Configuration").size(HeadlineSize::Small))
            .child(
                IconButton::new("lsp-config-info", IconName::Info)
                    .icon_size(IconSize::Custom(rems(1.5)))
                    .tooltip(Tooltip::text("Language Servers Documentation"))
                    .on_click(move |_, _, cx| cx.open_url("monyacode://docs/language-servers")),
            )
    }

    fn render_node(&self, settings: &NodeBinarySettings, theme: &Arc<Theme>, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .p_3()
            .gap_2()
            .border_1()
            .border_color(theme.colors().border)
            .rounded_md()
            .bg(theme.colors().element_background)
            .child(self.render_server_header(
                "Node.js".into(),
                settings.path.clone().unwrap_or("Node.js".to_string()).into(),
                Color::Default,
            ))
            .child(v_flex().gap_2().child(self.render_node_settings(settings, cx)))
    }

    fn render_node_settings(&self, settings: &NodeBinarySettings, cx: &Context<Self>) -> impl IntoElement {
        v_flex()
            .p_2()
            .gap_4()
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Switch::new(
                            "allow-binary-download-node",
                            ToggleState::from(settings.allow_binary_download),
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.toggle_allow_download_node(window, cx);
                        })),
                    )
                    .child(
                        v_flex().gap_1p5().child(Label::new("Download Node.js")).child(
                            Label::new("Allow the editor to download a Node.js binary if it can't find one.")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        ),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Switch::new(
                            "allow-binary-download-prettier",
                            ToggleState::from(settings.allow_prettier_download),
                        )
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.toggle_allow_download_prettier(window, cx);
                        })),
                    )
                    .child(
                        v_flex().gap_1p5().child(Label::new("Download Prettier")).child(
                            Label::new("Allow the editor to download the Prettier formatter unless already available.")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        ),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(
                        Switch::new("allow-npm-install", ToggleState::from(settings.allow_npm_install)).on_click(
                            cx.listener(move |this, _, window, cx| {
                                this.toggle_allow_npm_install(window, cx);
                            }),
                        ),
                    )
                    .child(
                        v_flex().gap_1p5().child(Label::new("Allow npm install")).child(
                            Label::new("Allow the editor to install NPM packages.")
                                .size(LabelSize::Small)
                                .color(Color::Muted),
                        ),
                    ),
            )
    }
}

impl Render for LspConfigView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.theme();
        let border_color = theme.colors().border;
        let element_background = theme.colors().element_background;
        let editor_background = theme.colors().editor_background;
        let global_state = cx.global::<LspConfigState>();
        let binary_statuses = &global_state.binary_statuses;
        let project_settings = ProjectSettings::get_global(cx);

        v_flex()
            .id("LspConfigView")
            .key_context("LspConfigView")
            .track_focus(&self.focus_handle(cx))
            .size_full()
            .gap_3()
            .p_4()
            .bg(editor_background)
            .overflow_hidden()
            .child(
                v_flex()
                    .id("lsp-config-view-servers")
                    .w_full()
                    .flex_1()
                    .gap_2()
                    .overflow_y_scroll()
                    .track_scroll(&self.scroll_handle)
                    .child(self.render_header())
                    .child(self.render_node(&project_settings.node, theme, cx))
                    .children(binary_statuses.iter().map(|(server_name, (status, _message))| {
                        let server_name = server_name.clone();

                        let binary_settings = project_settings
                            .lsp
                            .get(&server_name)
                            .and_then(|lsp| lsp.binary.clone())
                            .unwrap_or_default();
                        let (status_text, status_color) = Self::get_status_info(status);
                        let error_message = if let BinaryStatus::Failed { error } = &status {
                            Some(error.clone())
                        } else {
                            None
                        };

                        v_flex()
                            .p_3()
                            .gap_2()
                            .border_1()
                            .border_color(border_color)
                            .rounded_md()
                            .bg(element_background)
                            .child(self.render_server_header(server_name.0.clone(), status_text, status_color))
                            .child(v_flex().gap_2().child(self.render_binary_settings(
                                &server_name,
                                &binary_settings,
                                cx,
                            )))
                            .child(Self::render_error_message(&error_message))
                    })),
            )
            .vertical_scrollbar_for(&self.scroll_handle, window, cx)
    }
}

impl Focusable for LspConfigView {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Item for LspConfigView {
    type Event = EditorEvent;

    fn tab_content_text(&self, _detail: usize, _cx: &App) -> SharedString {
        SharedString::from("Language Servers")
    }
}
