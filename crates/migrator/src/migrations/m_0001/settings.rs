use anyhow::Result;
use serde_json::Value;

pub fn rename_status_bar_show(settings: &mut Value) -> Result<()> {
    if let Some(settings) = settings.as_object_mut()
        && let Some(status_bar) = settings.get_mut("status_bar").and_then(|v| v.as_object_mut())
        && let Some(show) = status_bar.remove("experimental.show")
    {
        status_bar.insert("show".to_string(), show);
    }
    Ok(())
}

pub fn removed_settings(settings: &mut Value) -> Result<()> {
    if let Some(settings) = settings.as_object_mut() {
        settings.remove("message_editor");
        settings.remove("notification_panel");
        settings.remove("project_name");
    };
    Ok(())
}
