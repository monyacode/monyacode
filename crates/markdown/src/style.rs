use std::sync::Arc;

use gpui::{FontWeight, Hsla, StyleRefinement, TextStyle, TextStyleRefinement};
use settings::Settings;
use theme::{SyntaxTheme, ThemeSettings};
use ui::{ActiveTheme, App, Color, Refineable, Styled, StyledTypography, Window, px, rems};

use crate::{HeadingLevelStyles, LinkStyleCallback};

#[derive(Clone, Default)]
pub struct MarkdownStyle {
    pub base_text_style: TextStyle,
    pub container_style: StyleRefinement,
    pub code_block: StyleRefinement,
    pub code_block_overflow_x_scroll: bool,
    pub inline_code: TextStyleRefinement,
    pub block_quote: TextStyleRefinement,
    pub link: TextStyleRefinement,
    pub link_callback: Option<LinkStyleCallback>,
    pub rule_color: Hsla,
    pub block_quote_border_color: Hsla,
    pub syntax: Arc<SyntaxTheme>,
    pub selection_background_color: Hsla,
    pub heading: StyleRefinement,
    pub heading_level_styles: Option<HeadingLevelStyles>,
    pub height_is_multiple_of_line_height: bool,
    pub prevent_mouse_interaction: bool,
    pub table_columns_min_size: bool,
}

impl MarkdownStyle {
    pub fn preview(_window: &Window, _cx: &App) -> Self {
        Self {
            base_text_style: Default::default(),
            container_style: Default::default(),
            code_block: Default::default(),
            code_block_overflow_x_scroll: false,
            inline_code: Default::default(),
            block_quote: Default::default(),
            link: Default::default(),
            link_callback: None,
            rule_color: Default::default(),
            block_quote_border_color: Default::default(),
            syntax: Arc::new(SyntaxTheme::default()),
            selection_background_color: Default::default(),
            heading: Default::default(),
            heading_level_styles: None,
            height_is_multiple_of_line_height: false,
            prevent_mouse_interaction: false,
            table_columns_min_size: false,
        }
    }

    pub fn hover(window: &Window, cx: &App) -> Self {
        let settings = ThemeSettings::get_global(cx);
        let ui_font_family = settings.ui_font.family.clone();
        let ui_font_features = settings.ui_font.features.clone();
        let ui_font_fallbacks = settings.ui_font.fallbacks.clone();
        let buffer_font_family = settings.buffer_font.family.clone();
        let buffer_font_features = settings.buffer_font.features.clone();
        let buffer_font_fallbacks = settings.buffer_font.fallbacks.clone();

        let mut base_text_style = window.text_style();
        base_text_style.refine(&TextStyleRefinement {
            font_family: Some(ui_font_family),
            font_features: Some(ui_font_features),
            font_fallbacks: ui_font_fallbacks,
            color: Some(cx.theme().colors().editor_foreground),
            ..Default::default()
        });
        MarkdownStyle {
            base_text_style,
            code_block: StyleRefinement::default()
                .my(rems(1.))
                .font_buffer(cx)
                .font_features(buffer_font_features.clone()),
            inline_code: TextStyleRefinement {
                background_color: Some(cx.theme().colors().background),
                font_family: Some(buffer_font_family),
                font_features: Some(buffer_font_features),
                font_fallbacks: buffer_font_fallbacks,
                ..Default::default()
            },
            rule_color: cx.theme().colors().border,
            block_quote_border_color: Color::Muted.color(cx),
            block_quote: TextStyleRefinement {
                color: Some(Color::Muted.color(cx)),
                ..Default::default()
            },
            link: TextStyleRefinement {
                color: Some(cx.theme().colors().editor_foreground),
                underline: Some(gpui::UnderlineStyle {
                    thickness: px(1.),
                    color: Some(cx.theme().colors().editor_foreground),
                    wavy: false,
                }),
                ..Default::default()
            },
            syntax: cx.theme().syntax().clone(),
            selection_background_color: cx.theme().colors().element_selection_background,
            heading: StyleRefinement::default()
                .font_weight(FontWeight::BOLD)
                .text_base()
                .mt(rems(1.))
                .mb_0(),
            table_columns_min_size: true,
            ..Default::default()
        }
    }

    pub fn diagnostics(window: &Window, cx: &App) -> Self {
        let settings = ThemeSettings::get_global(cx);
        let ui_font_family = settings.ui_font.family.clone();
        let ui_font_fallbacks = settings.ui_font.fallbacks.clone();
        let ui_font_features = settings.ui_font.features.clone();
        let buffer_font_family = settings.buffer_font.family.clone();
        let buffer_font_features = settings.buffer_font.features.clone();
        let buffer_font_fallbacks = settings.buffer_font.fallbacks.clone();

        let mut base_text_style = window.text_style();
        base_text_style.refine(&TextStyleRefinement {
            font_family: Some(ui_font_family),
            font_features: Some(ui_font_features),
            font_fallbacks: ui_font_fallbacks,
            color: Some(cx.theme().colors().editor_foreground),
            ..Default::default()
        });
        MarkdownStyle {
            base_text_style,
            code_block: StyleRefinement::default().my(rems(1.)).font_buffer(cx),
            inline_code: TextStyleRefinement {
                background_color: Some(cx.theme().colors().editor_background.opacity(0.5)),
                font_family: Some(buffer_font_family),
                font_features: Some(buffer_font_features),
                font_fallbacks: buffer_font_fallbacks,
                ..Default::default()
            },
            rule_color: cx.theme().colors().border,
            block_quote_border_color: Color::Muted.color(cx),
            block_quote: TextStyleRefinement {
                color: Some(Color::Muted.color(cx)),
                ..Default::default()
            },
            link: TextStyleRefinement {
                color: Some(cx.theme().colors().editor_foreground),
                underline: Some(gpui::UnderlineStyle {
                    thickness: px(1.),
                    color: Some(cx.theme().colors().editor_foreground),
                    wavy: false,
                }),
                ..Default::default()
            },
            syntax: cx.theme().syntax().clone(),
            selection_background_color: cx.theme().colors().element_selection_background,
            height_is_multiple_of_line_height: true,
            heading: StyleRefinement::default()
                .font_weight(FontWeight::BOLD)
                .text_base()
                .mb_0(),
            table_columns_min_size: true,
            ..Default::default()
        }
    }
}
