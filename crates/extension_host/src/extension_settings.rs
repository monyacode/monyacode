use extension::{DownloadFileCapability, ExtensionCapability, NpmInstallPackageCapability, ProcessExecCapability};
use settings::{RegisterSetting, Settings};

#[derive(Debug, Default, Clone, RegisterSetting)]
pub struct ExtensionSettings {
    pub granted_capabilities: Vec<ExtensionCapability>,
}

impl Settings for ExtensionSettings {
    fn from_settings(content: &settings::SettingsContent) -> Self {
        Self {
            granted_capabilities: content
                .extension
                .granted_extension_capabilities
                .clone()
                .unwrap_or_default()
                .into_iter()
                .map(|capability| match capability {
                    settings::ExtensionCapabilityContent::ProcessExec { command, args } => {
                        ExtensionCapability::ProcessExec(ProcessExecCapability { command, args })
                    }
                    settings::ExtensionCapabilityContent::DownloadFile { host, path } => {
                        ExtensionCapability::DownloadFile(DownloadFileCapability { host, path })
                    }
                    settings::ExtensionCapabilityContent::NpmInstallPackage { package } => {
                        ExtensionCapability::NpmInstallPackage(NpmInstallPackageCapability { package })
                    }
                })
                .collect(),
        }
    }
}
