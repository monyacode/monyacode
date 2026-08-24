use anyhow::Result;
use async_trait::async_trait;
use gpui::AsyncApp;
use language::{LspAdapter, LspAdapterDelegate, LspInstaller, Toolchain};
use lsp::{LanguageServerBinary, LanguageServerName};
use node_runtime::{NodeRuntime, VersionStrategy};
use project::ContextProviderWithTasks;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use task::{TaskTemplate, TaskTemplates, VariableName};
use util::{ResultExt, maybe};

const SERVER_PATH: &str = "node_modules/bash-language-server/out/";

pub struct BashLspAdapter {
    node: NodeRuntime,
}

fn server_binary_arguments(server_path: &Path) -> Vec<OsString> {
    vec![server_path.join("cli.js").into(), "start".into()]
}

impl BashLspAdapter {
    const SERVER_NAME: LanguageServerName = LanguageServerName::new_static("bash-language-server");
    const PACKAGE_NAME: &str = "bash-language-server";
    pub fn new(node: NodeRuntime) -> Self {
        BashLspAdapter { node }
    }
}

impl LspInstaller for BashLspAdapter {
    type BinaryVersion = String;

    async fn fetch_latest_server_version(
        &self,
        _: &dyn LspAdapterDelegate,
        _: bool,
        _: &mut AsyncApp,
    ) -> Result<String> {
        self.node.npm_package_latest_version("bash-language-server").await
    }

    async fn check_if_user_installed(
        &self,
        delegate: &dyn LspAdapterDelegate,
        _: Option<Toolchain>,
        _: &AsyncApp,
    ) -> Option<LanguageServerBinary> {
        let path = delegate.which(Self::SERVER_NAME.as_ref()).await?;
        let env = delegate.shell_env().await;

        Some(LanguageServerBinary {
            path,
            env: Some(env),
            arguments: vec!["start".into()],
        })
    }

    async fn fetch_server_binary(
        &self,
        latest_version: String,
        container_dir: PathBuf,
        _: &dyn LspAdapterDelegate,
    ) -> Result<LanguageServerBinary> {
        let server_path = container_dir.join(SERVER_PATH);

        self.node
            .npm_install_packages(&container_dir, &[(Self::PACKAGE_NAME, latest_version.as_str())])
            .await?;

        Ok(LanguageServerBinary {
            path: self.node.binary_path().await?,
            env: None,
            arguments: server_binary_arguments(&server_path),
        })
    }

    async fn check_if_version_installed(
        &self,
        version: &String,
        container_dir: &PathBuf,
        _: &dyn LspAdapterDelegate,
    ) -> Option<LanguageServerBinary> {
        let server_path = container_dir.join(SERVER_PATH);

        let should_install_language_server = self
            .node
            .should_install_npm_package(
                Self::PACKAGE_NAME,
                &server_path,
                container_dir,
                VersionStrategy::Latest(version),
            )
            .await;

        if should_install_language_server {
            None
        } else {
            Some(LanguageServerBinary {
                path: self.node.binary_path().await.ok()?,
                env: None,
                arguments: server_binary_arguments(&server_path),
            })
        }
    }

    async fn cached_server_binary(
        &self,
        container_dir: PathBuf,
        _: &dyn LspAdapterDelegate,
    ) -> Option<LanguageServerBinary> {
        get_cached_server_binary(container_dir, &self.node).await
    }
}

async fn get_cached_server_binary(container_dir: PathBuf, node: &NodeRuntime) -> Option<LanguageServerBinary> {
    maybe!(async {
        let server_path = container_dir.join(SERVER_PATH);
        anyhow::ensure!(server_path.exists(), "missing executable in directory {server_path:?}");
        Ok(LanguageServerBinary {
            path: node.binary_path().await?,
            env: None,
            arguments: server_binary_arguments(&server_path),
        })
    })
    .await
    .log_err()
}

#[async_trait(?Send)]
impl LspAdapter for BashLspAdapter {
    fn name(&self) -> LanguageServerName {
        Self::SERVER_NAME
    }
}

pub(super) fn bash_task_context() -> ContextProviderWithTasks {
    ContextProviderWithTasks::new(TaskTemplates(vec![
        TaskTemplate {
            label: "execute selection".to_owned(),
            command: VariableName::SelectedText.template_value(),
            ..TaskTemplate::default()
        },
        TaskTemplate {
            label: format!("run '{}'", VariableName::File.template_value()),
            command: VariableName::File.template_value(),
            ..TaskTemplate::default()
        },
    ]))
}

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, BorrowAppContext, Context, TestAppContext};
    use language::{AutoindentMode, Buffer};
    use settings::SettingsStore;
    use std::num::NonZeroU32;
    use unindent::Unindent;
    use util::test::marked_text_offsets;

    #[gpui::test]
    async fn test_bash_autoindent(cx: &mut TestAppContext) {
        cx.executor().set_block_on_ticks(usize::MAX..=usize::MAX);
        let language = crate::language("bash", tree_sitter_bash::LANGUAGE.into());
        cx.update(|cx| {
            let test_settings = SettingsStore::test(cx);
            cx.set_global(test_settings);
            cx.update_global::<SettingsStore, _>(|store, cx| {
                store.update_user_settings(cx, |s| s.project.all_languages.defaults.tab_size = NonZeroU32::new(2));
            });
        });

        cx.new(|cx| {
            let mut buffer = Buffer::local("", cx).with_language(language, cx);

            let expect_indents_to = |buffer: &mut Buffer, cx: &mut Context<Buffer>, input: &str, expected: &str| {
                buffer.edit([(0..buffer.len(), input)], Some(AutoindentMode::EachLine), cx);
                assert_eq!(buffer.text(), expected);
            };

            // Do not indent after shebang
            expect_indents_to(&mut buffer, cx, "#!/usr/bin/env bash\n#", "#!/usr/bin/env bash\n#");

            // indent function correctly
            expect_indents_to(
                &mut buffer,
                cx,
                "function name() {\necho \"Hello, World!\"\n}",
                "function name() {\n  echo \"Hello, World!\"\n}",
            );

            // indent if-else correctly
            expect_indents_to(
                &mut buffer,
                cx,
                "if true;then\nfoo\nelse\nbar\nfi",
                "if true;then\n  foo\nelse\n  bar\nfi",
            );

            // indent if-elif-else correctly
            expect_indents_to(
                &mut buffer,
                cx,
                "if true;then\nfoo\nelif true;then\nbar\nelse\nbar\nfi",
                "if true;then\n  foo\nelif true;then\n  bar\nelse\n  bar\nfi",
            );

            // indent case-when-else correctly
            expect_indents_to(
                &mut buffer,
                cx,
                "case $1 in\nfoo) echo \"Hello, World!\";;\n*) echo \"Unknown argument\";;\nesac",
                "case $1 in\n  foo) echo \"Hello, World!\";;\n  *) echo \"Unknown argument\";;\nesac",
            );

            // indent for-loop correctly
            expect_indents_to(
                &mut buffer,
                cx,
                "for i in {1..10};do\nfoo\ndone",
                "for i in {1..10};do\n  foo\ndone",
            );

            // indent while-loop correctly
            expect_indents_to(
                &mut buffer,
                cx,
                "while true; do\nfoo\ndone",
                "while true; do\n  foo\ndone",
            );

            // indent array correctly
            expect_indents_to(&mut buffer, cx, "array=(\n1\n2\n3\n)", "array=(\n  1\n  2\n  3\n)");

            // indents non-"function" function correctly
            expect_indents_to(
                &mut buffer,
                cx,
                "foo() {\necho \"Hello, World!\"\n}",
                "foo() {\n  echo \"Hello, World!\"\n}",
            );

            let (input, offsets) = marked_text_offsets(
                &r#"
                if foo; then
                  1ˇ
                else
                  3
                fi
                "#
                .unindent(),
            );

            buffer.edit([(0..buffer.len(), input)], None, cx);
            buffer.edit([(offsets[0]..offsets[0], "\n")], Some(AutoindentMode::EachLine), cx);
            buffer.edit(
                [(offsets[0] + 3..offsets[0] + 3, "elif")],
                Some(AutoindentMode::EachLine),
                cx,
            );
            let expected = r#"
                if foo; then
                  1
                elif
                else
                  3
                fi
                "#
            .unindent();

            pretty_assertions::assert_eq!(buffer.text(), expected);

            buffer
        });
    }
}
