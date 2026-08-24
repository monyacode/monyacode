// Based on https://github.com/zed-extensions/zig
// Author: Allan Calix <contact@acx.dev>
// License: Apache 2.0

use anyhow::Result;
use async_trait::async_trait;
use dap::{DapLocator, DebugRequest, adapters::DebugAdapterName};
use gpui::SharedString;
use serde_json::{Value, json};
use std::path::Path;
use task::{BuildTaskDefinition, DebugScenario, LaunchRequest, SpawnInTerminal, TaskTemplate};

const ZIG_TEST_EXE_BASENAME: &str = "zig_test";

pub(crate) struct ZigLocator;

#[async_trait]
impl DapLocator for ZigLocator {
    fn name(&self) -> SharedString {
        SharedString::new_static("zig-locator")
    }

    async fn create_scenario(
        &self,
        build_config: &TaskTemplate,
        resolved_label: &str,
        adapter: &DebugAdapterName,
    ) -> Option<DebugScenario> {
        if build_config.command != "zig" {
            return None;
        }

        let mut task_template = build_config.clone();
        let mut args_it = task_template.args.iter();
        match args_it.next() {
            Some(arg) if arg == "build" => match args_it.next() {
                Some(arg) if arg == "run" => {
                    task_template.label = "zig build".into();
                    task_template.command = "zig".into();
                    task_template.args = vec!["build".into()];
                }
                _ => return None,
            },
            Some(arg) if arg == "test" => {
                let test_exe_path = get_test_exe_path().unwrap();
                let mut args: Vec<String> = task_template.args.into_iter().map(|s| s.replace("\"", "'")).collect();
                args.push("--test-no-exec".into());
                args.push(format!("-femit-bin={test_exe_path}"));
                task_template.label = "zig test --test-no-exec".into();
                task_template.command = "zig".into();
                task_template.args = args;
            }
            _ => return None,
        };

        let config = if adapter.as_ref() == "CodeLLDB" {
            json!({"sourceLanguages": ["zig"]})
        } else {
            Value::Null
        };

        Some(DebugScenario {
            adapter: adapter.0.clone(),
            label: resolved_label.to_string().into(),
            build: Some(BuildTaskDefinition::Template {
                task_template,
                locator_name: Some(self.name()),
            }),
            config,
            tcp_connection: None,
        })
    }

    async fn run(&self, build_config: SpawnInTerminal) -> Result<DebugRequest> {
        let is_build = build_config.args.first().is_some_and(|arg| arg == "build");
        let is_test = build_config.args.first().is_some_and(|arg| arg == "test");
        anyhow::ensure!(is_build || is_test, "Unsupported build task");

        let program = if is_build {
            let Some(exec) = get_project_name(&build_config) else {
                anyhow::bail!("Failed to get project name");
            };
            format!("zig-out/bin/{exec}")
        } else {
            let Some(binary) = build_config.args.iter().find_map(|arg| {
                arg.strip_prefix("-femit-bin=").map(|arg| {
                    arg.split("=")
                        .nth(1)
                        .ok_or("Expected binary path in -femit-bin=")
                        .map(|path| path.trim_end_matches(".exe"))
                })
            }) else {
                anyhow::bail!("Failed to get project name");
            };
            match binary {
                Ok(binary) => binary.to_string(),
                Err(err) => anyhow::bail!(err),
            }
        };

        Ok(DebugRequest::Launch(LaunchRequest {
            program,
            cwd: build_config.cwd,
            args: vec![],
            env: build_config.env.into_iter().collect(),
        }))
    }
}

fn get_project_name(spawn: &SpawnInTerminal) -> Option<String> {
    spawn
        .cwd
        .as_ref()
        .and_then(|cwd| Some(Path::new(&cwd).file_name()?.to_string_lossy().into_owned()))
}

fn get_test_exe_path() -> Option<String> {
    let test_exe_dir = std::env::current_dir().ok()?;
    let name = format!(
        "{}_{}{}",
        ZIG_TEST_EXE_BASENAME,
        uuid::Uuid::new_v4(),
        if cfg!(windows) { ".exe" } else { "" },
    );
    Some(test_exe_dir.join(name).to_string_lossy().into_owned())
}
