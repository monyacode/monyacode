use settings::{RegisterSetting, Settings, SettingsContent};

#[derive(Copy, Clone, Debug, RegisterSetting)]
pub struct TitleBarSettings {
    pub show: bool,
    pub show_branch_icon: bool,
    pub show_branch_name: bool,
    pub show_project_items: bool,
    pub show_menus: bool,
}

impl Settings for TitleBarSettings {
    fn from_settings(s: &SettingsContent) -> Self {
        let content = s.title_bar.clone().unwrap();
        TitleBarSettings {
            show: content.show.unwrap(),
            show_branch_icon: content.show_branch_icon.unwrap(),
            show_branch_name: content.show_branch_name.unwrap(),
            show_project_items: content.show_project_items.unwrap(),
            show_menus: content.show_menus.unwrap(),
        }
    }
}
