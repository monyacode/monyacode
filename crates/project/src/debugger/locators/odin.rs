use anyhow::{Context as _, Result};
use async_trait::async_trait;
use base64::{Engine as _, engine::general_purpose};
use dap::{DapLocator, DebugRequest, adapters::DebugAdapterName};
use gpui::SharedString;
use serde_json;
use task::{BuildTaskDefinition, DebugScenario, LaunchRequest, ShellBuilder, SpawnInTerminal, TaskTemplate};

const ODIN_SCRIPT: &str = include_str!("odin/lldb.py");

pub(crate) struct OdinLocator;

#[async_trait]
impl DapLocator for OdinLocator {
    fn name(&self) -> SharedString {
        SharedString::new_static("odin-locator")
    }

    async fn create_scenario(
        &self,
        build_config: &TaskTemplate,
        resolved_label: &str,
        adapter: &DebugAdapterName,
    ) -> Option<DebugScenario> {
        let is_run = build_config.command == "odin" && build_config.args.first() == Some(&"run".into());
        let is_test = build_config.command == "odin" && build_config.args.first() == Some(&"test".into());
        if !is_run && !is_test {
            return None;
        }

        let mut task_template = build_config.clone();

        task_template.args[0] = "build".to_string();
        let exe_name = if cfg!(windows) {
            "debug_build.exe"
        } else {
            "debug_build"
        };
        task_template.args.push(format!("-out:{exe_name}"));
        if !task_template.args.contains(&"-debug".into()) {
            task_template.args.push("-debug".into());
        }

        if is_test {
            task_template.args.push("-build-mode:test".into());
        }

        task_template.label = if is_test {
            "odin debug test".into()
        } else {
            "odin debug build".into()
        };

        // Update the task labels. The resulting label will be displayed as-is in
        // the F4 Debug menu and will have "Debug: " prepended to the label when
        // shown in the test gutter.
        let label = if is_run {
            resolved_label
                .strip_prefix("run: ")
                .unwrap_or(&resolved_label)
                .to_string()
        } else {
            resolved_label
                .strip_prefix("test: ")
                .map(|suffix| format!("test {}", suffix))
                .unwrap_or_else(|| resolved_label.into())
        };

        let mut config_map = serde_json::Map::new();

        let encoded_script = general_purpose::STANDARD.encode(ODIN_SCRIPT);
        let exec_command = format!(
            "script import base64, types; odin = types.SimpleNamespace(); exec(base64.b64decode('{}').decode(), odin.__dict__); odin.__dict__['__lldb_init_module'](lldb.debugger, {{}})",
            encoded_script
        );

        config_map.insert("preRunCommands".to_string(), serde_json::json!(vec![exec_command]));

        let config = serde_json::Value::Object(config_map);

        Some(DebugScenario {
            adapter: adapter.0.clone(),
            label: label.into(),
            build: Some(BuildTaskDefinition::Template {
                task_template,
                locator_name: Some(self.name()),
            }),
            config,
            tcp_connection: None,
        })
    }

    async fn run(&self, build_config: SpawnInTerminal) -> Result<DebugRequest> {
        let cwd = build_config.cwd.clone().context("Couldn't get cwd from debug config")?;

        let builder = ShellBuilder::new(&build_config.shell, cfg!(windows)).non_interactive();
        let (program, args) = builder.build(Some("odin".into()), &build_config.args);

        let mut child = util::command::new_smol_command(program)
            .args(args)
            .envs(build_config.env.iter().map(|(k, v)| (k.clone(), v.clone())))
            .current_dir(&cwd)
            .spawn()?;

        let status = child.status().await?;
        anyhow::ensure!(status.success(), "Odin build command failed");

        let output_name = build_config
            .args
            .iter()
            .find_map(|arg| arg.strip_prefix("-out:"))
            .context("Failed to extract output binary name")?;

        let program = cwd.join(output_name).to_string_lossy().to_string();

        Ok(DebugRequest::Launch(LaunchRequest {
            program,
            cwd: Some(cwd),
            args: Vec::new(),
            env: build_config.env.into_iter().collect(),
        }))
    }
}
