// This crate was essentially pulled out verbatim from main `monyacode` crate to avoid having to run RustEmbed macro whenever monyacode has to be rebuilt. It saves a second or two on an incremental build.

use anyhow::Context as _;
use gpui::{App, AssetSource, Result, SharedString};
use rust_embed::RustEmbed;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(RustEmbed)]
#[folder = "../../assets"]
#[include = "fonts/**/*"]
#[include = "icons/**/*"]
#[include = "images/**/*"]
#[include = "themes/**/*"]
#[exclude = "themes/src/*"]
#[include = "sounds/**/*"]
#[include = "prompts/**/*"]
#[include = "*.toml"]
#[include = "*.md"]
#[include = "*.json"]
#[exclude = "*.DS_Store"]
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        Self::get(path)
            .map(|f| Some(f.data))
            .with_context(|| format!("loading asset at path {path:?}"))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| if p.starts_with(path) { Some(p.into()) } else { None })
            .collect())
    }
}

impl Assets {
    /// Populate the [`TextSystem`] of the given [`AppContext`] with all `.ttf` fonts in the `fonts` directory.
    pub fn load_fonts(&self, cx: &App) -> anyhow::Result<()> {
        let font_paths = self.list("fonts")?;
        let mut embedded_fonts = Vec::new();
        for font_path in font_paths {
            if font_path.ends_with(".ttf") {
                let font_bytes = cx
                    .asset_source()
                    .load(&font_path)?
                    .expect("Assets should never return None");
                embedded_fonts.push(font_bytes);
            }
        }

        cx.text_system().add_fonts(embedded_fonts)
    }

    pub fn load_test_fonts(&self, cx: &App) {
        cx.text_system()
            .add_fonts(vec![self.load("fonts/myna/Myna-Regular.ttf").unwrap().unwrap()])
            .unwrap()
    }

    pub fn random_greeting() -> SharedString {
        const FALLBACK: &str = "Monyatoring every suspicious semicolon.";
        let Some(file) = Self::get("random_greeting_dictionary.json") else {
            return FALLBACK.into();
        };
        let Ok(greetings) = serde_json::from_slice::<Vec<String>>(&file.data) else {
            return FALLBACK.into();
        };
        if greetings.is_empty() {
            return FALLBACK.into();
        }
        let index = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos() as usize % greetings.len())
            .unwrap_or(0);
        greetings[index].clone().into()
    }
}

#[derive(RustEmbed)]
#[folder = "../../docs"]
#[include = "*.md"]
#[include = "*.json"]
#[exclude = "*.DS_Store"]
pub struct Docs;

impl AssetSource for Docs {
    fn load(&self, path: &str) -> Result<Option<std::borrow::Cow<'static, [u8]>>> {
        Self::get(path)
            .map(|f| Some(f.data))
            .with_context(|| format!("loading docs at path {path:?}"))
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        Ok(Self::iter()
            .filter_map(|p| if p.starts_with(path) { Some(p.into()) } else { None })
            .collect())
    }
}

pub fn lookup_docs(path: &str) -> Option<rust_embed::EmbeddedFile> {
    if let Some(docs) = Docs::get(&path) {
        Some(docs)
    } else if let Some(docs) = Docs::get(&format!("{path}.md")) {
        Some(docs)
    } else {
        None
    }
}
