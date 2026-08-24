use anyhow::Context as _;
use gpui::{App, SharedString, UpdateGlobal};
use node_runtime::NodeRuntime;
use project::Fs;
use python::PyprojectTomlManifestProvider;
use rust::CargoManifestProvider;
use rust_embed::RustEmbed;
use settings::SettingsStore;
use smol::stream::StreamExt;
use std::{str, sync::Arc};
use util::{ResultExt, asset_str};

pub use language::*;

use crate::{
    json::JsonTaskProvider,
    python::{BasedPyrightLspAdapter, RuffLspAdapter},
};

mod bash;
mod c;
mod cpp;
mod css;
mod erlang;
mod eslint;
mod gleam;
mod go;
mod helpers;
mod html;
mod json;
mod lua;
mod nix;
mod odin;
mod opentofu;
mod package_json;
mod python;
mod rust;
mod tailwind;
mod tailwindcss;
mod toml;
mod typescript;
mod vtsls;
mod xml;
mod yaml;
mod zig;

pub(crate) use package_json::{PackageJson, PackageJsonData};

#[derive(RustEmbed)]
#[folder = "src/"]
#[exclude = "*.rs"]
struct LanguageDir;

/// A shared grammar for plain text, exposed for reuse by downstream crates.
#[cfg(feature = "tree-sitter-gitcommit")]
pub static LANGUAGE_GIT_COMMIT: std::sync::LazyLock<Arc<Language>> = std::sync::LazyLock::new(|| {
    Arc::new(Language::new(
        LanguageConfig {
            name: "Git Commit".into(),
            soft_wrap: Some(language::language_settings::SoftWrap::EditorWidth),
            matcher: LanguageMatcher {
                path_suffixes: vec!["COMMIT_EDITMSG".to_owned()],
                first_line_pattern: None,
            },
            line_comments: vec![Arc::from("#")],
            ..LanguageConfig::default()
        },
        Some(tree_sitter_gitcommit::LANGUAGE.into()),
    ))
});

pub fn init(languages: Arc<LanguageRegistry>, fs: Arc<dyn Fs>, node: NodeRuntime, cx: &mut App) {
    #[cfg(feature = "load-grammars")]
    languages.register_native_grammars([
        ("bash", tree_sitter_bash::LANGUAGE),
        ("c", tree_sitter_c::LANGUAGE),
        ("cpp", tree_sitter_cpp::LANGUAGE),
        ("css", tree_sitter_css::LANGUAGE),
        ("diff", tree_sitter_diff::LANGUAGE),
        ("erlang", tree_sitter_erlang::LANGUAGE),
        ("gleam", tree_sitter_gleam::LANGUAGE),
        ("go", tree_sitter_go::LANGUAGE),
        ("gomod", tree_sitter_go_mod::LANGUAGE),
        ("gowork", tree_sitter_gowork::LANGUAGE),
        ("html", tree_sitter_html::LANGUAGE),
        ("hcl", tree_sitter_hcl::LANGUAGE),
        ("jsdoc", tree_sitter_jsdoc::LANGUAGE),
        ("json", tree_sitter_json::LANGUAGE),
        ("jsonc", tree_sitter_json::LANGUAGE),
        ("lua", tree_sitter_lua::LANGUAGE),
        ("nix", tree_sitter_nix::LANGUAGE),
        ("markdown", tree_sitter_md::LANGUAGE),
        ("markdown-inline", tree_sitter_md::INLINE_LANGUAGE),
        ("odin", tree_sitter_odin::LANGUAGE),
        ("opentofu", tree_sitter_hcl::LANGUAGE),
        ("opentofu-vars", tree_sitter_hcl::LANGUAGE),
        ("python", tree_sitter_python::LANGUAGE),
        ("regex", tree_sitter_regex::LANGUAGE),
        ("roto", tree_sitter_roto::LANGUAGE),
        ("rust", tree_sitter_rust::LANGUAGE),
        ("scheme", tree_sitter_scheme::LANGUAGE),
        ("sql", tree_sitter_sql::LANGUAGE),
        ("toml", tree_sitter_toml::LANGUAGE),
        ("tsx", tree_sitter_typescript::LANGUAGE_TSX),
        ("typescript", tree_sitter_typescript::LANGUAGE_TYPESCRIPT),
        ("xml", tree_sitter_xml::LANGUAGE_XML),
        ("yaml", tree_sitter_yaml::LANGUAGE),
        ("zig", tree_sitter_zig::LANGUAGE),
        ("gitcommit", tree_sitter_gitcommit::LANGUAGE),
        ("git-rebase", tree_sitter_git_rebase::LANGUAGE),
    ]);

    let bash_lsp_adapter = Arc::new(bash::BashLspAdapter::new(node.clone()));
    let c_lsp_adapter = Arc::new(c::CLspAdapter);
    let css_lsp_adapter = Arc::new(css::CssLspAdapter::new(node.clone()));
    let eslint_adapter = Arc::new(eslint::EsLintLspAdapter::new(node.clone(), fs.clone()));
    let gleam_lsp_adapter = Arc::new(gleam::GleamLspAdapter);
    let gleam_context_provider = Arc::new(gleam::gleam_task_context());
    let go_context_provider = Arc::new(go::GoContextProvider);
    let go_lsp_adapter = Arc::new(go::GoLspAdapter);
    let html_lsp_adapter = Arc::new(html::HtmlLspAdapter::new(node.clone()));
    let superhtml_lsp_adapter = Arc::new(html::SuperhtmlLspAdapter);
    let json_context_provider = Arc::new(JsonTaskProvider);
    let erlang_ls_adapter = Arc::new(erlang::ErlangLsAdapter);
    let elp_adapter = Arc::new(erlang::ElpAdapter);
    let json_lsp_adapter = Arc::new(json::JsonLspAdapter::new(languages.clone(), node.clone()));
    let node_version_lsp_adapter = Arc::new(json::NodeVersionAdapter);
    let lua_lsp_adapter = Arc::new(lua::LuaLspAdapter);
    let nil_lsp_adapter = Arc::new(nix::NilLspAdapter);
    let nixd_lsp_adapter = Arc::new(nix::NixdLspAdapter);
    let odin_lsp_adapter = Arc::new(odin::OdinLspAdapter);
    let odin_context_provider = Arc::new(odin::odin_task_context());
    let opentofu_lsp_adapter = Arc::new(opentofu::OpenTofuLspAdapter);
    let py_lsp_adapter = Arc::new(python::PyLspAdapter::new());
    let ty_lsp_adapter = Arc::new(python::TyLspAdapter);
    let python_context_provider = Arc::new(python::PythonContextProvider);
    let python_lsp_adapter = Arc::new(python::PyrightLspAdapter::new(node.clone()));
    let basedpyright_lsp_adapter = Arc::new(BasedPyrightLspAdapter::new(node.clone()));
    let ruff_lsp_adapter = Arc::new(RuffLspAdapter::new(fs.clone()));
    let python_toolchain_provider = Arc::new(python::PythonToolchainProvider);
    let rust_context_provider = Arc::new(rust::RustContextProvider);
    let rust_lsp_adapter = Arc::new(rust::RustLspAdapter);
    let tailwind_adapter = Arc::new(tailwind::TailwindLspAdapter::new(node.clone()));
    let tailwindcss_adapter = Arc::new(tailwindcss::TailwindCssLspAdapter::new(node.clone()));
    let toml_lsp_adapter = Arc::new(toml::TomlLspAdapter);
    let typescript_context = Arc::new(typescript::TypeScriptContextProvider::new(fs.clone()));
    let typescript_lsp_adapter = Arc::new(typescript::TypeScriptLspAdapter::new(node.clone(), fs.clone()));
    let vtsls_adapter = Arc::new(vtsls::VtslsLspAdapter::new(node.clone(), fs.clone()));
    let xml_lsp_adapter = Arc::new(xml::XmlLspAdapter);
    let yaml_lsp_adapter = Arc::new(yaml::YamlLspAdapter::new(node));
    let zig_lsp_adapter = Arc::new(zig::ZigLspAdapter);
    let zig_context_provider = Arc::new(zig::zig_task_context());

    let built_in_languages = [
        LanguageInfo {
            name: "bash",
            adapters: vec![bash_lsp_adapter],
            context: Some(Arc::new(bash::bash_task_context())),
            ..Default::default()
        },
        LanguageInfo {
            name: "c",
            adapters: vec![c_lsp_adapter.clone()],
            ..Default::default()
        },
        LanguageInfo {
            name: "cpp",
            adapters: vec![c_lsp_adapter],
            ..Default::default()
        },
        LanguageInfo {
            name: "css",
            adapters: vec![css_lsp_adapter],
            ..Default::default()
        },
        LanguageInfo {
            name: "diff",
            adapters: vec![],
            ..Default::default()
        },
        LanguageInfo {
            name: "erlang",
            adapters: vec![erlang_ls_adapter, elp_adapter],
            ..Default::default()
        },
        LanguageInfo {
            name: "gleam",
            adapters: vec![gleam_lsp_adapter],
            context: Some(gleam_context_provider),
            manifest_name: Some(SharedString::new_static("gleam.toml").into()),
            ..Default::default()
        },
        LanguageInfo {
            name: "go",
            adapters: vec![go_lsp_adapter.clone()],
            context: Some(go_context_provider.clone()),
            ..Default::default()
        },
        LanguageInfo {
            name: "gomod",
            adapters: vec![go_lsp_adapter.clone()],
            context: Some(go_context_provider.clone()),
            ..Default::default()
        },
        LanguageInfo {
            name: "gowork",
            adapters: vec![go_lsp_adapter],
            context: Some(go_context_provider),
            ..Default::default()
        },
        LanguageInfo {
            name: "hcl",
            adapters: vec![],
            ..Default::default()
        },
        LanguageInfo {
            name: "html",
            adapters: vec![superhtml_lsp_adapter, html_lsp_adapter],
            ..Default::default()
        },
        LanguageInfo {
            name: "json",
            adapters: vec![json_lsp_adapter.clone(), node_version_lsp_adapter],
            context: Some(json_context_provider.clone()),
            ..Default::default()
        },
        LanguageInfo {
            name: "jsonc",
            adapters: vec![json_lsp_adapter],
            context: Some(json_context_provider),
            ..Default::default()
        },
        LanguageInfo {
            name: "lua",
            adapters: vec![lua_lsp_adapter],
            ..Default::default()
        },
        LanguageInfo {
            name: "nix",
            adapters: vec![nil_lsp_adapter, nixd_lsp_adapter],
            ..Default::default()
        },
        LanguageInfo {
            name: "markdown",
            adapters: vec![],
            ..Default::default()
        },
        LanguageInfo {
            name: "markdown-inline",
            adapters: vec![],
            ..Default::default()
        },
        LanguageInfo {
            name: "odin",
            adapters: vec![odin_lsp_adapter],
            context: Some(odin_context_provider),
            ..Default::default()
        },
        LanguageInfo {
            name: "opentofu",
            adapters: vec![opentofu_lsp_adapter.clone()],
            ..Default::default()
        },
        LanguageInfo {
            name: "opentofu-vars",
            adapters: vec![opentofu_lsp_adapter],
            ..Default::default()
        },
        LanguageInfo {
            name: "python",
            adapters: vec![basedpyright_lsp_adapter, ruff_lsp_adapter],
            context: Some(python_context_provider),
            toolchain: Some(python_toolchain_provider),
            manifest_name: Some(SharedString::new_static("pyproject.toml").into()),
        },
        LanguageInfo {
            name: "scheme",
            adapters: vec![],
            ..Default::default()
        },
        LanguageInfo {
            name: "rust",
            adapters: vec![rust_lsp_adapter],
            context: Some(rust_context_provider),
            manifest_name: Some(SharedString::new_static("Cargo.toml").into()),
            ..Default::default()
        },
        LanguageInfo {
            name: "sql",
            adapters: vec![],
            ..Default::default()
        },
        LanguageInfo {
            name: "toml",
            adapters: vec![toml_lsp_adapter],
            ..Default::default()
        },
        LanguageInfo {
            name: "tsx",
            adapters: vec![typescript_lsp_adapter.clone(), vtsls_adapter.clone()],
            context: Some(typescript_context.clone()),
            ..Default::default()
        },
        LanguageInfo {
            name: "typescript",
            adapters: vec![typescript_lsp_adapter.clone(), vtsls_adapter.clone()],
            context: Some(typescript_context.clone()),
            ..Default::default()
        },
        LanguageInfo {
            name: "javascript",
            adapters: vec![typescript_lsp_adapter.clone(), vtsls_adapter.clone()],
            context: Some(typescript_context),
            ..Default::default()
        },
        LanguageInfo {
            name: "jsdoc",
            adapters: vec![typescript_lsp_adapter.clone(), vtsls_adapter.clone()],
            ..Default::default()
        },
        LanguageInfo {
            name: "regex",
            adapters: vec![],
            ..Default::default()
        },
        LanguageInfo {
            name: "xml",
            adapters: vec![xml_lsp_adapter],
            ..Default::default()
        },
        LanguageInfo {
            name: "yaml",
            adapters: vec![yaml_lsp_adapter],
            ..Default::default()
        },
        LanguageInfo {
            name: "zig",
            adapters: vec![zig_lsp_adapter],
            context: Some(zig_context_provider),
            manifest_name: Some(SharedString::new_static("build.zig.zon").into()),
            ..Default::default()
        },
        LanguageInfo {
            name: "gitcommit",
            ..Default::default()
        },
        LanguageInfo {
            name: "git-rebase",
            ..Default::default()
        },
        LanguageInfo {
            name: "monyacode-keybind-context",
            ..Default::default()
        },
        LanguageInfo {
            name: "roto",
            ..Default::default()
        },
    ];

    for registration in built_in_languages {
        register_language(
            &languages,
            registration.name,
            registration.adapters,
            registration.context,
            registration.toolchain,
            registration.manifest_name,
        );
    }

    // Register globally available language servers.
    //
    // This will allow users to add support for a built-in language server (e.g., Tailwind)
    // for a given language via the `language_servers` setting:
    //
    // ```json
    // {
    //   "languages": {
    //     "My Language": {
    //       "language_servers": ["tailwindcss-language-server", "..."]
    //     }
    //   }
    // }
    // ```
    languages.register_available_lsp_adapter(
        LanguageServerName("tailwindcss-language-server".into()),
        tailwind_adapter.clone(),
    );
    languages.register_available_lsp_adapter(
        LanguageServerName("tailwindcss-intellisense-css".into()),
        tailwindcss_adapter,
    );
    languages.register_available_lsp_adapter(LanguageServerName("eslint".into()), eslint_adapter.clone());
    languages.register_available_lsp_adapter(LanguageServerName("vtsls".into()), vtsls_adapter);
    languages.register_available_lsp_adapter(
        LanguageServerName("typescript-language-server".into()),
        typescript_lsp_adapter,
    );

    languages.register_available_lsp_adapter(python_lsp_adapter.name(), python_lsp_adapter);
    languages.register_available_lsp_adapter(py_lsp_adapter.name(), py_lsp_adapter);
    languages.register_available_lsp_adapter(ty_lsp_adapter.name(), ty_lsp_adapter);
    // Register Tailwind for the existing languages that should have it by default.
    //
    // This can be driven by the `language_servers` setting once we have a way for
    // extensions to provide their own default value for that setting.
    let tailwind_languages = [
        "Astro",
        "CSS",
        "ERB",
        "HTML+ERB",
        "HEEX",
        "HTML",
        "JavaScript",
        "TypeScript",
        "PHP",
        "Svelte",
        "TSX",
        "Vue.js",
    ];

    for language in tailwind_languages {
        languages.register_lsp_adapter(language.into(), tailwind_adapter.clone());
    }

    let eslint_languages = ["TSX", "TypeScript", "JavaScript", "Vue.js", "Svelte"];
    for language in eslint_languages {
        languages.register_lsp_adapter(language.into(), eslint_adapter.clone());
    }

    let mut subscription = languages.subscribe();
    let mut prev_language_settings = languages.language_settings();

    cx.spawn(async move |cx| {
        while subscription.next().await.is_some() {
            let language_settings = languages.language_settings();
            if language_settings != prev_language_settings {
                cx.update(|cx| {
                    SettingsStore::update_global(cx, |settings, cx| {
                        settings
                            .set_extension_settings(
                                settings::ExtensionsSettingsContent {
                                    all_languages: language_settings.clone(),
                                },
                                cx,
                            )
                            .log_err();
                    });
                })?;
                prev_language_settings = language_settings;
            }
        }
        anyhow::Ok(())
    })
    .detach();
    let manifest_providers: [Arc<dyn ManifestProvider>; 2] = [
        Arc::from(CargoManifestProvider),
        Arc::from(PyprojectTomlManifestProvider),
    ];
    for provider in manifest_providers {
        project::ManifestProvidersStore::global(cx).register(provider);
    }
}

#[derive(Default)]
struct LanguageInfo {
    name: &'static str,
    adapters: Vec<Arc<dyn LspAdapter>>,
    context: Option<Arc<dyn ContextProvider>>,
    toolchain: Option<Arc<dyn ToolchainLister>>,
    manifest_name: Option<ManifestName>,
}

fn register_language(
    languages: &LanguageRegistry,
    name: &'static str,
    adapters: Vec<Arc<dyn LspAdapter>>,
    context: Option<Arc<dyn ContextProvider>>,
    toolchain: Option<Arc<dyn ToolchainLister>>,
    manifest_name: Option<ManifestName>,
) {
    let config = load_config(name);
    for adapter in adapters {
        languages.register_lsp_adapter(config.name.clone(), adapter);
    }
    languages.register_language(
        config.name.clone(),
        config.grammar.clone(),
        config.matcher.clone(),
        config.hidden,
        manifest_name.clone(),
        Arc::new(move || {
            Ok(LoadedLanguage {
                config: config.clone(),
                queries: load_queries(name),
                context_provider: context.clone(),
                toolchain_provider: toolchain.clone(),
                manifest_name: manifest_name.clone(),
            })
        }),
    );
}

#[cfg(any(test, feature = "test-support"))]
pub fn language(name: &str, grammar: tree_sitter::Language) -> Arc<Language> {
    Arc::new(
        Language::new(load_config(name), Some(grammar))
            .with_queries(load_queries(name))
            .unwrap(),
    )
}

fn load_config(name: &str) -> LanguageConfig {
    let config_toml = String::from_utf8(
        LanguageDir::get(&format!("{}/config.toml", name))
            .unwrap_or_else(|| panic!("missing config for language {:?}", name))
            .data
            .to_vec(),
    )
    .unwrap();

    #[allow(unused_mut)]
    let mut config: LanguageConfig = ::toml::from_str(&config_toml)
        .with_context(|| format!("failed to load config.toml for language {name:?}"))
        .unwrap();

    #[cfg(not(any(feature = "load-grammars", test)))]
    {
        config = LanguageConfig {
            name: config.name,
            matcher: config.matcher,
            jsx_tag_auto_close: config.jsx_tag_auto_close,
            ..Default::default()
        }
    }

    config
}

fn load_queries(name: &str) -> LanguageQueries {
    let mut result = LanguageQueries::default();
    for path in LanguageDir::iter() {
        if let Some(remainder) = path.strip_prefix(name).and_then(|p| p.strip_prefix('/')) {
            if !remainder.ends_with(".scm") {
                continue;
            }
            for (name, query) in QUERY_FILENAME_PREFIXES {
                if remainder.starts_with(name) {
                    let contents = asset_str::<LanguageDir>(path.as_ref());
                    match query(&mut result) {
                        None => *query(&mut result) = Some(contents),
                        Some(r) => r.to_mut().push_str(contents.as_ref()),
                    }
                }
            }
        }
    }
    result
}
