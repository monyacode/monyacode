mod application_menu;
pub mod platform_title_bar;
mod platforms;
mod system_window_tabs;
pub mod title_bar_settings;

#[cfg(feature = "stories")]
mod stories;

use crate::{
    application_menu::{ApplicationMenu, show_menus},
    platform_title_bar::PlatformTitleBar,
    system_window_tabs::SystemWindowTabs,
};

#[cfg(not(target_os = "macos"))]
use crate::application_menu::{ActivateDirection, ActivateMenuLeft, ActivateMenuRight, OpenApplicationMenu};

use app_actions::{OpenRecent, OpenRemote};
use gpui::{
    Action, AnyElement, App, Context, Corner, Element, Entity, Focusable, InteractiveElement, IntoElement, MouseButton,
    ParentElement, Render, Styled, Subscription, WeakEntity, Window,
};
use project::{Project, git_store::GitStoreEvent};
use remote::RemoteConnectionOptions;
use settings::Settings;
use theme::ActiveTheme;
use title_bar_settings::TitleBarSettings;
use ui::{
    Button, ButtonLike, ButtonStyle, ContextMenu, Icon, IconName, IconSize, IconWithIndicator, Indicator, PopoverMenu,
    Tooltip, h_flex, prelude::*,
};
use workspace::Workspace;

#[cfg(feature = "stories")]
pub use stories::*;

const MAX_PROJECT_NAME_LENGTH: usize = 40;
const MAX_BRANCH_NAME_LENGTH: usize = 40;
const MAX_SHORT_SHA_LENGTH: usize = 8;

fn apply_title_bar_visibility(workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    let should_show = TitleBarSettings::get_global(cx).show;

    if should_show {
        if workspace.titlebar_item().is_none() {
            let item = cx.new(|cx| TitleBar::new("title-bar", workspace, window, cx));
            workspace.set_titlebar_item(item.into(), window, cx);
        }
    } else {
        workspace.clear_titlebar_item(window, cx);
    }

    #[cfg(target_os = "macos")]
    window.set_traffic_light_visible(should_show);
}

pub fn init(cx: &mut App) {
    SystemWindowTabs::init(cx);

    cx.observe_new(|workspace: &mut Workspace, window, cx| {
        let Some(window) = window else {
            log::debug!("titlebar: Didn't get a window for some reason");
            return;
        };
        apply_title_bar_visibility(workspace, window, cx);
        // React to settings changes
        cx.observe_global_in::<settings::SettingsStore>(window, |workspace, window, cx| {
            apply_title_bar_visibility(workspace, window, cx);
        })
        .detach();

        #[cfg(not(target_os = "macos"))]
        workspace.register_action(|workspace, action: &OpenApplicationMenu, window, cx| {
            if let Some(titlebar) = workspace
                .titlebar_item()
                .and_then(|item| item.downcast::<TitleBar>().ok())
            {
                titlebar.update(cx, |titlebar, cx| {
                    if let Some(ref menu) = titlebar.application_menu {
                        menu.update(cx, |menu, cx| menu.open_menu(action, window, cx));
                    }
                });
            }
        });

        #[cfg(not(target_os = "macos"))]
        workspace.register_action(|workspace, _: &ActivateMenuRight, window, cx| {
            if let Some(titlebar) = workspace
                .titlebar_item()
                .and_then(|item| item.downcast::<TitleBar>().ok())
            {
                titlebar.update(cx, |titlebar, cx| {
                    if let Some(ref menu) = titlebar.application_menu {
                        menu.update(cx, |menu, cx| {
                            menu.navigate_menus_in_direction(ActivateDirection::Right, window, cx)
                        });
                    }
                });
            }
        });

        #[cfg(not(target_os = "macos"))]
        workspace.register_action(|workspace, _: &ActivateMenuLeft, window, cx| {
            if let Some(titlebar) = workspace
                .titlebar_item()
                .and_then(|item| item.downcast::<TitleBar>().ok())
            {
                titlebar.update(cx, |titlebar, cx| {
                    if let Some(ref menu) = titlebar.application_menu {
                        menu.update(cx, |menu, cx| {
                            menu.navigate_menus_in_direction(ActivateDirection::Left, window, cx)
                        });
                    }
                });
            }
        });
    })
    .detach();
}

pub struct TitleBar {
    platform_titlebar: Entity<PlatformTitleBar>,
    project: Entity<Project>,
    workspace: WeakEntity<Workspace>,
    application_menu: Option<Entity<ApplicationMenu>>,
    _subscriptions: Vec<Subscription>,
}

impl Render for TitleBar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let title_bar_settings = *TitleBarSettings::get_global(cx);

        let show_menus = show_menus(cx);

        let mut children = Vec::new();

        children.push(
            h_flex()
                .gap_1()
                .map(|title_bar| {
                    let mut render_project_items =
                        title_bar_settings.show_branch_name || title_bar_settings.show_project_items;
                    title_bar
                        .when_some(
                            self.application_menu.clone().filter(|_| !show_menus),
                            |title_bar, menu| {
                                render_project_items &= !menu.update(cx, |menu, cx| menu.all_menus_shown(cx));
                                title_bar.child(menu)
                            },
                        )
                        .when(render_project_items, |title_bar| {
                            title_bar
                                .when(title_bar_settings.show_project_items, |title_bar| {
                                    title_bar
                                        .children(self.render_project_host(cx))
                                        .child(self.render_project_name(cx))
                                })
                                .when(title_bar_settings.show_branch_name, |title_bar| {
                                    title_bar.children(self.render_project_repo(cx))
                                })
                        })
                })
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .into_any_element(),
        );

        children.push(
            h_flex()
                .pr_1()
                .gap_1()
                .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                .child(self.render_app_menu_button(cx))
                .into_any_element(),
        );

        if show_menus {
            self.platform_titlebar.update(cx, |this, _| {
                this.set_children(self.application_menu.clone().map(|menu| menu.into_any_element()));
            });

            let height = PlatformTitleBar::height(window);
            let title_bar_color = self.platform_titlebar.update(cx, |platform_titlebar, cx| {
                platform_titlebar.title_bar_color(window, cx)
            });

            v_flex()
                .w_full()
                .child(self.platform_titlebar.clone().into_any_element())
                .child(
                    h_flex()
                        .bg(title_bar_color)
                        .h(height)
                        .pl_2()
                        .justify_between()
                        .w_full()
                        .children(children),
                )
                .into_any_element()
        } else {
            self.platform_titlebar.update(cx, |this, _| {
                this.set_children(children);
            });
            self.platform_titlebar.clone().into_any_element()
        }
    }
}

impl TitleBar {
    pub fn new(id: impl Into<ElementId>, workspace: &Workspace, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let project = workspace.project().clone();
        let git_store = project.read(cx).git_store().clone();

        let platform_style = PlatformStyle::platform();
        let application_menu = match platform_style {
            PlatformStyle::Mac => {
                if option_env!("MONYACODE_USE_CROSS_PLATFORM_MENU").is_some() {
                    Some(cx.new(|cx| ApplicationMenu::new(window, cx)))
                } else {
                    None
                }
            }
            PlatformStyle::Linux | PlatformStyle::Windows => Some(cx.new(|cx| ApplicationMenu::new(window, cx))),
        };

        let mut subscriptions = Vec::new();
        subscriptions.push(cx.observe(&workspace.weak_handle().upgrade().unwrap(), |_, _, cx| cx.notify()));
        subscriptions.push(cx.subscribe(&project, |_, _, _: &project::Event, cx| cx.notify()));
        subscriptions.push(cx.observe_window_activation(window, Self::window_activation_changed));
        subscriptions.push(cx.subscribe(&git_store, move |_, _, event, cx| match event {
            GitStoreEvent::ActiveRepositoryChanged(_) | GitStoreEvent::RepositoryUpdated(_, _, true) => {
                cx.notify();
            }
            _ => {}
        }));

        let platform_titlebar = cx.new(|cx| PlatformTitleBar::new(id, cx));

        Self {
            platform_titlebar,
            application_menu,
            workspace: workspace.weak_handle(),
            project,
            _subscriptions: subscriptions,
        }
    }

    fn project_name(&self, cx: &Context<Self>) -> Option<SharedString> {
        self.project
            .read(cx)
            .visible_worktrees(cx)
            .map(|worktree| SharedString::new(worktree.read(cx).root_name_str()))
            .next()
    }

    fn render_remote_project_connection(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let options = self.project.read(cx).remote_connection_options(cx)?;
        let host: SharedString = options.display_name().into();

        let (nickname, icon) = match options {
            RemoteConnectionOptions::Ssh(options) => (options.nickname.map(|nick| nick.into()), IconName::Server),
            RemoteConnectionOptions::Wsl(_) => (None, IconName::Linux),
        };
        let nickname = nickname.unwrap_or_else(|| host.clone());

        let (indicator_color, meta) = match self.project.read(cx).remote_connection_state(cx)? {
            remote::ConnectionState::Connecting => (Color::Info, format!("Connecting to: {host}")),
            remote::ConnectionState::Connected => (Color::Success, format!("Connected to: {host}")),
            remote::ConnectionState::HeartbeatMissed => (
                Color::Warning,
                format!("Connection attempt to {host} missed. Retrying..."),
            ),
            remote::ConnectionState::Reconnecting => {
                (Color::Warning, format!("Lost connection to {host}. Reconnecting..."))
            }
            remote::ConnectionState::Disconnected => (Color::Error, format!("Disconnected from {host}")),
        };

        let icon_color = match self.project.read(cx).remote_connection_state(cx)? {
            remote::ConnectionState::Connecting => Color::Info,
            remote::ConnectionState::Connected => Color::Default,
            remote::ConnectionState::HeartbeatMissed => Color::Warning,
            remote::ConnectionState::Reconnecting => Color::Warning,
            remote::ConnectionState::Disconnected => Color::Error,
        };

        let meta = SharedString::from(meta);

        Some(
            ButtonLike::new("ssh-server-icon")
                .child(
                    h_flex()
                        .gap_2()
                        .max_w_32()
                        .child(
                            IconWithIndicator::new(
                                Icon::new(icon).size(IconSize::Small).color(icon_color),
                                Some(Indicator::dot().color(indicator_color)),
                            )
                            .indicator_border_color(Some(cx.theme().colors().title_bar_background))
                            .into_any_element(),
                        )
                        .child(Label::new(nickname).size(LabelSize::Small).truncate()),
                )
                .tooltip(move |_window, cx| {
                    Tooltip::with_meta(
                        "Remote Project",
                        Some(&OpenRemote {
                            from_existing_connection: false,
                            create_new_window: false,
                        }),
                        meta.clone(),
                        cx,
                    )
                })
                .on_click(|_, window, cx| {
                    window.dispatch_action(
                        OpenRemote {
                            from_existing_connection: false,
                            create_new_window: false,
                        }
                        .boxed_clone(),
                        cx,
                    );
                })
                .into_any_element(),
        )
    }

    pub fn render_project_host(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        if self.project.read(cx).is_via_remote_server() {
            return self.render_remote_project_connection(cx);
        }

        if self.project.read(cx).is_disconnected(cx) {
            return Some(
                Button::new("disconnected", "Disconnected")
                    .disabled(true)
                    .color(Color::Disabled)
                    .style(ButtonStyle::Subtle)
                    .label_size(LabelSize::Small)
                    .into_any_element(),
            );
        }

        None
    }

    pub fn render_project_name(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let name = self.project_name(cx);
        let is_project_selected = name.is_some();
        let name = if let Some(name) = name {
            util::truncate_and_trailoff(&name, MAX_PROJECT_NAME_LENGTH)
        } else {
            "Open Recent Project".to_string()
        };

        Button::new("project_name_trigger", name)
            .when(!is_project_selected, |b| b.color(Color::Muted))
            .style(ButtonStyle::Subtle)
            .label_size(LabelSize::Small)
            .tooltip(move |_window, cx| {
                Tooltip::for_action(
                    "Recent Projects",
                    &app_actions::OpenRecent {
                        create_new_window: false,
                    },
                    cx,
                )
            })
            .on_click(cx.listener(move |_, _, window, cx| {
                window.dispatch_action(
                    OpenRecent {
                        create_new_window: false,
                    }
                    .boxed_clone(),
                    cx,
                );
            }))
    }

    pub fn render_project_repo(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
        let settings = TitleBarSettings::get_global(cx);
        let repository = self.project.read(cx).active_repository(cx)?;
        let repository_count = self.project.read(cx).repositories(cx).len();
        let workspace = self.workspace.upgrade()?;
        let repo = repository.read(cx);
        let branch_name = repo
            .branch
            .as_ref()
            .map(|branch| branch.name())
            .map(|name| util::truncate_and_trailoff(name, MAX_BRANCH_NAME_LENGTH))
            .or_else(|| {
                repo.head_commit
                    .as_ref()
                    .map(|commit| commit.sha.chars().take(MAX_SHORT_SHA_LENGTH).collect::<String>())
            })?;
        let project_name = self.project_name(cx);
        let repo_name = repo
            .work_directory_abs_path
            .file_name()
            .and_then(|name| name.to_str())
            .map(SharedString::new);
        let show_repo_name = repository_count > 1 && repo.branch.is_some() && repo_name != project_name;
        let branch_name = if let Some(repo_name) = repo_name.filter(|_| show_repo_name) {
            format!("{repo_name}/{branch_name}")
        } else {
            branch_name
        };

        Some(
            Button::new("project_branch_trigger", branch_name)
                .color(Color::Muted)
                .style(ButtonStyle::Subtle)
                .label_size(LabelSize::Small)
                .tooltip(move |_window, cx| {
                    Tooltip::with_meta(
                        "Recent Branches",
                        Some(&app_actions::git::Branch),
                        "Local branches only",
                        cx,
                    )
                })
                .on_click(move |_, window, cx| {
                    let _ = workspace.update(cx, |this, cx| {
                        window.focus(&this.active_pane().focus_handle(cx), cx);
                        window.dispatch_action(app_actions::git::Branch.boxed_clone(), cx);
                    });
                })
                .when(settings.show_branch_icon, |branch_button| {
                    let (icon, icon_color) = {
                        let status = repo.status_summary();
                        let tracked = status.index + status.worktree;
                        if status.conflict > 0 {
                            (IconName::Warning, Color::VersionControlConflict)
                        } else if tracked.modified > 0 {
                            (IconName::SquareDot, Color::VersionControlModified)
                        } else if tracked.added > 0 || status.untracked > 0 {
                            (IconName::SquarePlus, Color::VersionControlAdded)
                        } else if tracked.deleted > 0 {
                            (IconName::SquareMinus, Color::VersionControlDeleted)
                        } else {
                            (IconName::GitBranch, Color::Muted)
                        }
                    };

                    branch_button
                        .icon(icon)
                        .icon_position(IconPosition::Start)
                        .icon_color(icon_color)
                        .icon_size(IconSize::Indicator)
                }),
        )
    }

    fn window_activation_changed(&mut self, _window: &mut Window, _cx: &mut Context<Self>) {}

    pub fn render_app_menu_button(&mut self, _cx: &mut Context<Self>) -> impl Element {
        PopoverMenu::new("user-menu")
            .anchor(Corner::TopRight)
            .menu(move |window, cx| {
                ContextMenu::build(window, cx, |menu, _, _cx| {
                    menu.action("Documentation", app_actions::OpenDocs.boxed_clone())
                        .separator()
                        .action("Settings", app_actions::OpenSettings.boxed_clone())
                        .action("Keymap", Box::new(app_actions::OpenKeymap))
                        .action("Extensions", app_actions::Extensions::default().boxed_clone())
                        .action("Themes…", app_actions::theme_selector::Toggle::default().boxed_clone())
                        .action(
                            "Icon Themes…",
                            app_actions::icon_theme_selector::Toggle::default().boxed_clone(),
                        )
                })
                .into()
            })
            .map(|this| {
                this.trigger_with_tooltip(
                    IconButton::new("user-menu", IconName::Sword).icon_size(IconSize::Small),
                    Tooltip::text("Toggle User Menu"),
                )
            })
            .anchor(gpui::Corner::TopRight)
    }
}
