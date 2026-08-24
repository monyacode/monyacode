use anyhow::Context as _;
use gpui::{App, SerializedThreadTaskTimings};
use log::info;
use smol::stream::StreamExt;
use std::{thread::ThreadId, time::Duration};
use util::ResultExt;

use crate::STARTUP_TIME;

pub fn init(cx: &mut App) {
    monitor_hangs(cx);
}

fn monitor_hangs(cx: &App) {
    let main_thread_id = std::thread::current().id();

    let foreground_executor = cx.foreground_executor();
    let background_executor = cx.background_executor();

    // 3 seconds hang
    let (mut tx, mut rx) = futures::channel::mpsc::channel(3);
    foreground_executor
        .spawn(async move { while (rx.next().await).is_some() {} })
        .detach();

    background_executor
        .spawn({
            let background_executor = background_executor.clone();
            async move {
                let mut hang_time = None;

                let mut hanging = false;
                loop {
                    background_executor.timer(Duration::from_secs(1)).await;
                    match tx.try_send(()) {
                        Ok(_) => {
                            hang_time = None;
                            hanging = false;
                            continue;
                        }
                        Err(e) => {
                            let is_full = e.into_send_error().is_full();
                            if is_full && !hanging {
                                hanging = true;
                                hang_time = Some(chrono::Local::now());
                            }

                            if is_full {
                                save_hang_trace(main_thread_id, &background_executor, hang_time.unwrap());
                            }
                        }
                    }
                }
            }
        })
        .detach();
}

fn save_hang_trace(
    main_thread_id: ThreadId,
    background_executor: &gpui::BackgroundExecutor,
    hang_time: chrono::DateTime<chrono::Local>,
) {
    let thread_timings = background_executor.dispatcher.get_all_timings();
    let thread_timings = thread_timings
        .into_iter()
        .map(|mut timings| {
            if timings.thread_id == main_thread_id {
                timings.thread_name = Some("main".to_string());
            }

            SerializedThreadTaskTimings::convert(*STARTUP_TIME.get().unwrap(), timings)
        })
        .collect::<Vec<_>>();

    let trace_path = paths::hang_traces_dir().join(&format!("hang-{}.miniprof", hang_time.format("%Y-%m-%d_%H-%M-%S")));

    let Some(timings) = serde_json::to_string(&thread_timings)
        .context("hang timings serialization")
        .log_err()
    else {
        return;
    };

    std::fs::write(&trace_path, timings)
        .context("hang trace file writing")
        .log_err();

    info!("hang detected, trace file saved at: {}", trace_path.display());
}
