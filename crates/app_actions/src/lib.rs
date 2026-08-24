use gpui::{Action, actions};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// If the editor binary doesn't use anything in this crate, it will be optimized away
// and the actions won't initialize. So we just provide an empty initialization function
// to be called from main.
//
// These may provide relevant context:
// https://github.com/rust-lang/rust/issues/47384
// https://github.com/mmastrac/rust-ctor/issues/280
pub fn init() {}

/// Opens a URL in the system's default web browser.
#[derive(Clone, PartialEq, Deserialize, JsonSchema, Action)]
#[action(namespace = monyacode)]
#[serde(deny_unknown_fields)]
pub struct OpenBrowser {
    pub url: String,
}

/// Opens a monyacode:// URL within the application.
#[derive(Clone, PartialEq, Deserialize, JsonSchema, Action)]
#[action(namespace = monyacode)]
#[serde(deny_unknown_fields)]
pub struct OpenMonyaCodeUrl {
    pub url: String,
}

/// Opens the keymap to either add a keybinding or change an existing one
#[derive(PartialEq, Clone, Default, Action, JsonSchema, Serialize, Deserialize)]
#[action(namespace = monyacode, no_json, no_register)]
pub struct ChangeKeybinding {
    pub action: String,
}

actions!(
    monyacode,
    [
        /// Opens the settings editor.
        #[action(deprecated_aliases = ["app_actions::OpenSettingsEditor"])]
        OpenSettings,
        /// Opens the settings JSON file.
        #[action(deprecated_aliases = ["app_actions::OpenSettings"])]
        OpenSettingsFile,
        /// Opens project-specific settings.
        #[action(deprecated_aliases = ["app_actions::OpenProjectSettings"])]
        OpenProjectSettings,
        /// Opens the default keymap file.
        OpenDefaultKeymap,
        /// Opens the user keymap file.
        #[action(deprecated_aliases = ["app_actions::OpenKeymap"])]
        OpenKeymapFile,
        /// Opens the keymap editor.
        #[action(deprecated_aliases = ["app_actions::OpenKeymapEditor"])]
        OpenKeymap,
        /// Opens server settings.
        OpenServerSettings,
        /// Quits the application.
        Quit,
        /// Shows information about MonyaCode.
        About,
        /// Opens the documentation website.
        OpenDocs,
        /// Views open source licenses.
        OpenLicenses,
        /// Opens the performance profiler.
        OpenPerformanceProfiler,
        /// Opens the onboarding view.
        OpenOnboarding,
        /// Toggles the title bar visibility
        ToggleTitleBar
    ]
);

#[derive(PartialEq, Clone, Copy, Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionCategoryFilter {
    Themes,
    IconThemes,
    Languages,
    Grammars,
    LanguageServers,
    IndexedDocsProviders,
    Snippets,
    DebugAdapters,
}

/// Opens the extensions management interface.
#[derive(PartialEq, Clone, Default, Debug, Deserialize, JsonSchema, Action)]
#[action(namespace = monyacode)]
#[serde(deny_unknown_fields)]
pub struct Extensions {
    /// Filters the extensions page down to extensions that are in the specified category.
    #[serde(default)]
    pub category_filter: Option<ExtensionCategoryFilter>,
    /// Focuses just the extension with the specified ID.
    #[serde(default)]
    pub id: Option<String>,
}

/// Decreases the font size in the editor buffer.
#[derive(PartialEq, Clone, Default, Debug, Deserialize, JsonSchema, Action)]
#[action(namespace = monyacode)]
#[serde(deny_unknown_fields)]
pub struct DecreaseBufferFontSize {
    #[serde(default)]
    pub persist: bool,
}

/// Increases the font size in the editor buffer.
#[derive(PartialEq, Clone, Default, Debug, Deserialize, JsonSchema, Action)]
#[action(namespace = monyacode)]
#[serde(deny_unknown_fields)]
pub struct IncreaseBufferFontSize {
    #[serde(default)]
    pub persist: bool,
}

/// Open the settings at a specific setting
#[derive(PartialEq, Clone, Debug, Deserialize, JsonSchema, Action)]
#[action(namespace = monyacode)]
#[serde(deny_unknown_fields)]
pub struct OpenSettingsAt {
    /// A path to a specific setting (e.g. `theme.mode`)
    pub path: String,
}

/// Open the documentation viewer at the specific page
#[derive(PartialEq, Clone, Debug, Deserialize, JsonSchema, Action)]
#[action(namespace = monyacode)]
#[serde(deny_unknown_fields)]
pub struct OpenDocsAt {
    /// A path to a documentation page
    pub path: String,
}

/// Resets the buffer font size to the default value.
#[derive(PartialEq, Clone, Default, Debug, Deserialize, JsonSchema, Action)]
#[action(namespace = monyacode)]
#[serde(deny_unknown_fields)]
pub struct ResetBufferFontSize {
    #[serde(default)]
    pub persist: bool,
}

/// Decreases the font size of the user interface.
#[derive(PartialEq, Clone, Default, Debug, Deserialize, JsonSchema, Action)]
#[action(namespace = monyacode)]
#[serde(deny_unknown_fields)]
pub struct DecreaseUiFontSize {
    #[serde(default)]
    pub persist: bool,
}

/// Increases the font size of the user interface.
#[derive(PartialEq, Clone, Default, Debug, Deserialize, JsonSchema, Action)]
#[action(namespace = monyacode)]
#[serde(deny_unknown_fields)]
pub struct IncreaseUiFontSize {
    #[serde(default)]
    pub persist: bool,
}

/// Resets the UI font size to the default value.
#[derive(PartialEq, Clone, Default, Debug, Deserialize, JsonSchema, Action)]
#[action(namespace = monyacode)]
#[serde(deny_unknown_fields)]
pub struct ResetUiFontSize {
    #[serde(default)]
    pub persist: bool,
}

/// Resets all zoom levels (UI and buffer font sizes) to their default values.
#[derive(PartialEq, Clone, Default, Debug, Deserialize, JsonSchema, Action)]
#[action(namespace = monyacode)]
#[serde(deny_unknown_fields)]
pub struct ResetAllZoom {
    #[serde(default)]
    pub persist: bool,
}

pub mod dev {
    use gpui::actions;

    actions!(
        dev,
        [
            /// Toggles the developer inspector for debugging UI elements.
            ToggleInspector
        ]
    );
}

pub mod workspace {
    use gpui::actions;

    actions!(
        workspace,
        [
            #[action(deprecated_aliases = ["editor::CopyPath", "outline_panel::CopyPath", "project_panel::CopyPath"])]
            CopyPath,
            #[action(deprecated_aliases = ["editor::CopyRelativePath", "outline_panel::CopyRelativePath", "project_panel::CopyRelativePath"])]
            CopyRelativePath,
            /// Opens the selected file with the system's default application.
            #[action(deprecated_aliases = ["project_panel::OpenWithSystem"])]
            OpenWithSystem,
        ]
    );
}

pub mod git {
    use gpui::actions;

    actions!(
        git,
        [
            /// Checks out a different git branch.
            CheckoutBranch,
            /// Switches to a different git branch.
            Switch,
            /// Selects a different repository.
            SelectRepo,
            /// Filter remotes.
            FilterRemotes,
            /// Create a git remote.
            CreateRemote,
            /// Opens the git branch selector.
            #[action(deprecated_aliases = ["branches::OpenRecent"])]
            Branch,
            /// Opens the git stash selector.
            ViewStash,
            /// Opens the git worktree selector.
            Worktree,
            /// Creates a pull request for the current branch.
            CreatePullRequest,
            /// Opens the Git Graph view
            OpenGraph,
        ]
    );
}

pub mod toast {
    use gpui::actions;

    actions!(
        toast,
        [
            /// Runs the action associated with a toast notification.
            RunAction
        ]
    );
}

pub mod command_palette {
    use gpui::actions;

    actions!(
        command_palette,
        [
            /// Toggles the command palette.
            Toggle,
        ]
    );
}

pub mod docs_search {
    use gpui::actions;

    actions!(
        docs_search,
        [
            /// Opens the Documentation Search popup.
            Toggle,
        ]
    );
}

pub mod quick_search {
    use gpui::actions;

    actions!(
        quick_search,
        [
            /// Opens the Quick Search popup.
            Toggle,
        ]
    );
}

pub mod project_panel {
    use gpui::actions;

    actions!(
        project_panel,
        [
            /// Toggles the project panel
            Toggle,
            /// Toggles focus on the project panel.
            ToggleFocus
        ]
    );
}

pub mod theme_selector {
    use gpui::Action;
    use schemars::JsonSchema;
    use serde::Deserialize;

    /// Toggles the theme selector interface.
    #[derive(PartialEq, Clone, Default, Debug, Deserialize, JsonSchema, Action)]
    #[action(namespace = theme_selector)]
    #[serde(deny_unknown_fields)]
    pub struct Toggle {
        /// A list of theme names to filter the theme selector down to.
        pub themes_filter: Option<Vec<String>>,
    }
}

pub mod icon_theme_selector {
    use gpui::Action;
    use schemars::JsonSchema;
    use serde::Deserialize;

    /// Toggles the icon theme selector interface.
    #[derive(PartialEq, Clone, Default, Debug, Deserialize, JsonSchema, Action)]
    #[action(namespace = icon_theme_selector)]
    #[serde(deny_unknown_fields)]
    pub struct Toggle {
        /// A list of icon theme names to filter the theme selector down to.
        pub themes_filter: Option<Vec<String>>,
    }
}

pub mod settings_profile_selector {
    use gpui::Action;
    use schemars::JsonSchema;
    use serde::Deserialize;

    #[derive(PartialEq, Clone, Default, Debug, Deserialize, JsonSchema, Action)]
    #[action(namespace = settings_profile_selector)]
    pub struct Toggle;
}

/// Opens the recent projects interface.
#[derive(PartialEq, Clone, Deserialize, Default, JsonSchema, Action)]
#[action(namespace = projects)]
#[serde(deny_unknown_fields)]
pub struct OpenRecent {
    #[serde(default)]
    pub create_new_window: bool,
}

/// Creates a project from a selected template.
#[derive(PartialEq, Clone, Deserialize, Default, JsonSchema, Action)]
#[action(namespace = projects)]
#[serde(deny_unknown_fields)]
pub struct OpenRemote {
    #[serde(default)]
    pub from_existing_connection: bool,
    #[serde(default)]
    pub create_new_window: bool,
}

/// Where to spawn the task in the UI.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RevealTarget {
    /// In the central pane group, "main" editor area.
    Center,
    /// In the terminal dock, "regular" terminal items' place.
    #[default]
    Dock,
}

/// Spawns a task with name or opens tasks modal.
#[derive(Debug, PartialEq, Clone, Deserialize, JsonSchema, Action)]
#[action(namespace = task)]
#[serde(untagged)]
pub enum Spawn {
    /// Spawns a task by the name given.
    ByName {
        task_name: String,
        #[serde(default)]
        reveal_target: Option<RevealTarget>,
    },
    /// Spawns a task by the name given.
    ByTag {
        task_tag: String,
        #[serde(default)]
        reveal_target: Option<RevealTarget>,
    },
    /// Spawns a task via modal's selection.
    ViaModal {
        /// Selected task's `reveal_target` property override.
        #[serde(default)]
        reveal_target: Option<RevealTarget>,
    },
}

impl Spawn {
    pub fn modal() -> Self {
        Self::ViaModal { reveal_target: None }
    }
}

/// Reruns the last task.
#[derive(PartialEq, Clone, Deserialize, Default, JsonSchema, Action)]
#[action(namespace = task)]
#[serde(deny_unknown_fields)]
pub struct Rerun {
    /// Controls whether the task context is reevaluated prior to execution of a task.
    /// If it is not, environment variables such as MONYACODE_COLUMN, MONYACODE_FILE are gonna be the same as in the last execution of a task
    /// If it is, these variables will be updated to reflect current state of editor at the time task::Rerun is executed.
    /// default: false
    #[serde(default)]
    pub reevaluate_context: bool,
    /// Overrides `allow_concurrent_runs` property of the task being reran.
    /// Default: null
    #[serde(default)]
    pub allow_concurrent_runs: Option<bool>,
    /// Overrides `use_new_terminal` property of the task being reran.
    /// Default: null
    #[serde(default)]
    pub use_new_terminal: Option<bool>,

    /// If present, rerun the task with this ID, otherwise rerun the last task.
    #[serde(skip)]
    pub task_id: Option<String>,
}

pub mod outline {
    use std::sync::OnceLock;

    use gpui::{AnyView, App, Window, actions};

    actions!(
        outline,
        [
            #[action(name = "Toggle")]
            ToggleOutline
        ]
    );
    /// A pointer to outline::toggle function, exposed here to sewer the breadcrumbs <-> outline dependency.
    pub static TOGGLE_OUTLINE: OnceLock<fn(AnyView, &mut Window, &mut App)> = OnceLock::new();
}

actions!(
    debug_panel,
    [
        /// Toggles the debug panel.
        Toggle,
        /// Toggles focus on the debug panel.
        ToggleFocus
    ]
);
actions!(
    debugger,
    [
        /// Toggles the enabled state of a breakpoint.
        ToggleEnableBreakpoint,
        /// Removes a breakpoint.
        UnsetBreakpoint,
        /// Opens the project debug tasks configuration.
        OpenProjectDebugTasks,
    ]
);

pub mod vim {
    use gpui::actions;

    actions!(
        vim,
        [
            /// Opens the default keymap file.
            OpenDefaultKeymap
        ]
    );
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WslConnectionOptions {
    pub distro_name: String,
    pub user: Option<String>,
}

#[cfg(target_os = "windows")]
pub mod wsl_actions {
    use gpui::Action;
    use schemars::JsonSchema;
    use serde::Deserialize;

    /// Opens a folder inside Wsl.
    #[derive(PartialEq, Clone, Deserialize, Default, JsonSchema, Action)]
    #[action(namespace = projects)]
    #[serde(deny_unknown_fields)]
    pub struct OpenFolderInWsl {
        #[serde(default)]
        pub create_new_window: bool,
    }

    /// Open a wsl distro.
    #[derive(PartialEq, Clone, Deserialize, Default, JsonSchema, Action)]
    #[action(namespace = projects)]
    #[serde(deny_unknown_fields)]
    pub struct OpenWsl {
        #[serde(default)]
        pub create_new_window: bool,
    }
}

pub mod preview {
    pub mod markdown {
        use gpui::actions;

        actions!(
            markdown,
            [
                /// Opens a markdown preview for the current file.
                OpenPreview,
                /// Opens a markdown preview in a split pane.
                OpenPreviewToTheSide,
            ]
        );
    }

    pub mod svg {
        use gpui::actions;

        actions!(
            svg,
            [
                /// Opens an SVG preview for the current file.
                OpenPreview,
                /// Opens an SVG preview in a split pane.
                OpenPreviewToTheSide,
            ]
        );
    }
}
