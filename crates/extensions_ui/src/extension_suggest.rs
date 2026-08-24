use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

use db::kvp::KEY_VALUE_STORE;
use editor::Editor;
use gpui::{AppContext as _, Context, Entity, SharedString, Window};
use language::Buffer;
use ui::prelude::*;
use util::rel_path::RelPath;
use workspace::notifications::simple_message_notification::MessageNotification;
use workspace::{Workspace, notifications::NotificationId};

use crate::InstallExtensionFromGit;

const SUGGESTIONS_BY_EXTENSION_ID: &[(&str, &[&str])] = &[
    ("ada", &["ada"]),
    ("asm", &["asm"]),
    ("astro", &["astro"]),
    ("bash", &["sh"]),
    ("beancount", &["beancount"]),
    ("clojure", &["bb", "clj", "cljc", "cljs", "edn"]),
    ("cobol", &["cbl", "cpy"]),
    ("neocmake", &["CMakeLists.txt", "cmake"]),
    ("csharp", &["cs"]),
    ("cython", &["pyx", "pxd", "pxi"]),
    ("d", &["d"]),
    ("dart", &["dart"]),
    ("dockerfile", &["Dockerfile"]),
    ("elisp", &["el"]),
    ("elixir", &["ex", "exs", "heex"]),
    ("elm", &["elm"]),
    ("fish", &["fish"]),
    ("fortran", &["f90", "f95", "f03", "f", "for"]),
    ("fsharp", &["fs", "fsi", "fsx"]),
    (
        "git-firefly",
        &[
            ".gitconfig",
            ".gitignore",
            "COMMIT_EDITMSG",
            "EDIT_DESCRIPTION",
            "MERGE_MSG",
            "NOTES_EDITMSG",
            "TAG_EDITMSG",
            "git-rebase-todo",
        ],
    ),
    ("glsl", &["vert", "frag"]),
    ("graphql", &["gql", "graphql"]),
    ("graphviz", &["dot"]),
    ("haskell", &["hs"]),
    ("haxe", &["hx", "hxml"]),
    ("java", &["java"]),
    ("jai", &["jai"]),
    ("justfile", &["justfile"]),
    ("kotlin", &["kt"]),
    ("latex", &["tex"]),
    ("log", &["log"]),
    ("make", &["Makefile"]),
    ("nim", &["nim"]),
    ("nix", &["nix"]),
    ("nu", &["nu"]),
    ("ocaml", &["ml", "mli"]),
    ("php", &["php"]),
    ("powershell", &["ps1", "psm1"]),
    ("prisma", &["prisma"]),
    ("proto", &["proto"]),
    ("purescript", &["purs"]),
    ("r", &["r", "R"]),
    ("racket", &["rkt"]),
    ("rescript", &["res", "resi"]),
    ("rst", &["rst"]),
    ("ruby", &["rb", "erb"]),
    ("scss", &["scss"]),
    ("squirrel", &["nut"]),
    ("svelte", &["svelte"]),
    ("swift", &["swift"]),
    ("templ", &["templ"]),
    ("terraform", &["tf", "tfvars", "hcl"]),
    ("typst", &["typ"]),
    ("uiua", &["ua"]),
    ("vue", &["vue"]),
    ("wgsl", &["wgsl"]),
    ("wit", &["wit"]),
];

const EXTENSION_URL: &[(&str, &str)] = &[
    ("ada", "https://github.com/wisn/zed-ada-language"),
    ("asm", "https://github.com/DevBlocky/zed-asm"),
    ("astro", "https://github.com/zed-extensions/astro"),
    ("bash", "https://github.com/zed-extensions/bash"),
    ("beancount", "https://github.com/zed-extensions/beancount"),
    ("clojure", "https://github.com/zed-extensions/clojure"),
    ("neocmake", "https://github.com/k0tran/zed_neocmake"),
    ("cobol", "https://github.com/willswire/zed-cobol"),
    ("csharp", "https://github.com/zed-extensions/csharp"),
    ("cython", "https://github.com/lgeiger/zed-cython"),
    ("d", "https://github.com/staysail/zed-d"),
    ("dart", "https://github.com/zed-extensions/dart"),
    ("dockerfile", "https://github.com/zed-extensions/dockerfile"),
    ("elisp", "https://github.com/JosephTLyons/zed-elisp"),
    ("elixir", "https://github.com/zed-extensions/elixir"),
    ("elm", "https://github.com/zed-extensions/elm"),
    ("fish", "https://github.com/hasit/zed-fish"),
    ("fortran", "https://github.com/Xavier-Maruff/zed-fortran"),
    ("fsharp", "https://github.com/nathanjcollins/zed-fsharp"),
    ("git-firefly", "https://github.com/zed-extensions/git_firefly"),
    ("glsl", "https://codeberg.org/GramEditor/glsl-extension"),
    ("graphql", "https://github.com/11bit/zed-extension-graphql"),
    ("graphviz", "https://github.com/gabeins/zed-graphviz"),
    ("haskell", "https://github.com/zed-extensions/haskell"),
    ("haxe", "https://github.com/Frixuu/Zed-Haxe"),
    ("java", "https://github.com/zed-extensions/java"),
    ("jai", "https://github.com/seg4lt/zed-jai"),
    ("justfile", "https://github.com/jackTabsCode/zed-just"),
    ("kotlin", "https://github.com/zed-extensions/kotlin"),
    ("latex", "https://github.com/rzukic/zed-latex"),
    ("log", "https://github.com/zed-extensions/log"),
    ("luau", "https://github.com/4teapo/zed-luau"),
    ("make", "https://github.com/caius/zed-make"),
    ("nim", "https://github.com/foxoman/zed-nim"),
    ("nix", "https://github.com/zed-extensions/nix"),
    ("nu", "https://github.com/zed-extensions/nu"),
    ("ocaml", "https://github.com/zed-extensions/ocaml"),
    ("php", "https://github.com/zed-extensions/php"),
    ("powershell", "https://github.com/wingyplus/zed-powershell"),
    ("prisma", "https://github.com/zed-extensions/prisma"),
    ("proto", "https://codeberg.org/GramEditor/proto-extension"),
    ("purescript", "https://github.com/zed-extensions/purescript"),
    ("r", "https://github.com/ocsmit/zed-r"),
    ("racket", "https://github.com/zed-extensions/racket"),
    ("rescript", "https://github.com/humaans/rescript-zed"),
    ("rst", "https://github.com/elmarco/zed-rst"),
    ("ruby", "https://github.com/zed-extensions/ruby"),
    ("scss", "https://github.com/bajrangCoder/zed-scss"),
    ("sql", "https://github.com/zed-extensions/sql"),
    ("squirrel", "https://github.com/mnshdw/squirrel-lsp-zed"),
    ("svelte", "https://github.com/zed-extensions/svelte"),
    ("swift", "https://github.com/zed-extensions/swift"),
    ("templ", "https://github.com/makifdb/zed-templ"),
    ("terraform", "https://github.com/zed-extensions/terraform"),
    ("typst", "https://github.com/weethet/typst.zed"),
    ("uiua", "https://github.com/zed-extensions/uiua"),
    ("vue", "https://github.com/zed-extensions/vue"),
    ("wgsl", "https://github.com/luan/zed-wgsl"),
    ("wit", "https://github.com/valentinegb/zed-wit"),
];

fn suggested_extensions() -> &'static HashMap<&'static str, Arc<str>> {
    static SUGGESTIONS_BY_PATH_SUFFIX: OnceLock<HashMap<&str, Arc<str>>> = OnceLock::new();
    SUGGESTIONS_BY_PATH_SUFFIX.get_or_init(|| {
        SUGGESTIONS_BY_EXTENSION_ID
            .iter()
            .flat_map(|(name, path_suffixes)| {
                let name = Arc::<str>::from(*name);
                path_suffixes.iter().map(move |suffix| (*suffix, name.clone()))
            })
            .collect()
    })
}

#[derive(Debug, PartialEq, Eq, Clone)]
struct SuggestedExtension {
    pub extension_id: Arc<str>,
    pub file_name_or_extension: Arc<str>,
}

/// Returns the suggested extension for the given [`Path`].
fn suggested_extension(path: &RelPath) -> Option<SuggestedExtension> {
    let file_extension: Option<Arc<str>> = path.extension().map(|extension| extension.into());
    let file_name: Option<Arc<str>> = path.file_name().map(|name| name.into());

    let (file_name_or_extension, extension_id) = None
        // We suggest against file names first, as these suggestions will be more
        // specific than ones based on the file extension.
        .or_else(|| {
            file_name.clone().zip(
                file_name
                    .as_deref()
                    .and_then(|file_name| suggested_extensions().get(file_name)),
            )
        })
        .or_else(|| {
            file_extension.clone().zip(
                file_extension
                    .as_deref()
                    .and_then(|file_extension| suggested_extensions().get(file_extension)),
            )
        })?;

    Some(SuggestedExtension {
        extension_id: extension_id.clone(),
        file_name_or_extension,
    })
}

fn language_extension_key(extension_id: &str) -> String {
    format!("{}_extension_suggest", extension_id)
}

fn pretty_url(url: &str) -> &str {
    let url = url.strip_prefix("https://").unwrap_or(url);
    let url = url.strip_prefix("http://").unwrap_or(url);
    let url = url.strip_prefix("www.").unwrap_or(url);
    url.strip_suffix(".git").unwrap_or(url)
}

pub(crate) fn suggest(buffer: Entity<Buffer>, window: &mut Window, cx: &mut Context<Workspace>) {
    let Some(file) = buffer.read(cx).file().cloned() else {
        return;
    };

    let Some(SuggestedExtension {
        extension_id,
        file_name_or_extension,
    }) = suggested_extension(file.path())
    else {
        return;
    };
    log::info!("Found suggested extension {}", extension_id);

    let key = language_extension_key(&extension_id);
    let Ok(None) = KEY_VALUE_STORE.read_kvp(&key) else {
        log::info!("Found installed extension for key {}", key);
        return;
    };

    cx.on_next_frame(window, move |workspace, _, cx| {
        let Some(editor) = workspace.active_item_as::<Editor>(cx) else {
            log::info!("suggest {}: No matching editor in workspace", key);
            return;
        };

        if editor.read(cx).buffer().read(cx).as_singleton().as_ref() != Some(&buffer) {
            log::info!("suggest {}: Not the active buffer", key);
            return;
        }

        struct ExtensionSuggestionNotification;

        let notification_id = NotificationId::composite::<ExtensionSuggestionNotification>(
            SharedString::from(extension_id.clone()),
        );

        let Some(url) = EXTENSION_URL.iter().find(|eu| *eu.0 == *extension_id) else {
            log::info!("No URL found for {}", extension_id);
            return;
        };
        let url = url.1;
        log::info!("URL for {}: {}", extension_id, url);

        workspace.show_notification(notification_id, cx, |cx| {
            cx.new(move |cx| {
                MessageNotification::new(
                    if extension_id == file_name_or_extension {
                        format!(
                            "Install an extension for '{}' files?\nInstallation from source URL requires rustup to be installed.",
                            extension_id,
                        )
                    } else {
                        format!(
                            "Install an extension for '{}' ('{}') files?\nInstallation from source URL requires rustup to be installed.",
                            extension_id, file_name_or_extension,
                        )
                    },
                    cx,
                )
                .primary_message("Install")
                .primary_icon(IconName::Check)
                .primary_icon_color(Color::Success)
                .primary_on_click(
                    move |window, cx| {
                        window.dispatch_action(
                            Box::new(InstallExtensionFromGit { url: url.into() }),
                            cx,
                        );
                    }
                )
                .secondary_message("No")
                .secondary_icon(IconName::Close)
                .secondary_icon_color(Color::Error)
                .secondary_on_click(move |_window, cx| {
                    let key = language_extension_key(&extension_id);
                    db::write_and_log(cx, move || {
                        KEY_VALUE_STORE.write_kvp(key, "dismissed".to_string())
                    });
                })
                .more_info_message(pretty_url(url))
                .more_info_url(url)
            })
        });
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use util::rel_path::rel_path;

    #[test]
    pub fn test_suggested_extension() {
        assert_eq!(
            suggested_extension(rel_path("Dockerfile")),
            Some(SuggestedExtension {
                extension_id: "dockerfile".into(),
                file_name_or_extension: "Dockerfile".into()
            })
        );
        assert_eq!(
            suggested_extension(rel_path("a/b/c/d/.gitignore")),
            Some(SuggestedExtension {
                extension_id: "git-firefly".into(),
                file_name_or_extension: ".gitignore".into()
            })
        );
    }
}
