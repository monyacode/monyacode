use std::ops::Range;

use gpui::Entity;
use language::Buffer;
use project::ProjectPath;
use text::Anchor;
use ui::{Context, IntoElement, Pixels, Render, Window, div};

pub const SEARCH_DEBOUNCE_MS: u64 = 100;
pub const DEFAULT_RESULTS_HEIGHT: f32 = 180.0;
pub const DEFAULT_PREVIEW_HEIGHT: f32 = 280.0;
pub const MIN_PANEL_HEIGHT: f32 = 80.0;
pub const MAX_PREVIEW_HEIGHT: f32 = 600.0;

#[derive(Clone, Debug)]
pub struct SearchMatch {
    pub path: ProjectPath,
    pub buffer: Entity<Buffer>,
    pub anchor_range: Range<Anchor>,
    pub range: Range<usize>,
    pub relative_range: Range<usize>,
    pub line_text: String,
    pub line_number: u32,
}

#[derive(Clone, Copy)]
pub struct SearchDrag {
    pub mouse_start: gpui::Point<Pixels>,
    pub offset_start: gpui::Point<Pixels>,
}

#[derive(Clone, Copy)]
pub struct ResizeDrag {
    pub mouse_start_y: Pixels,
    pub results_height_start: Pixels,
    pub preview_height_start: Pixels,
}

#[derive(Clone, Copy)]
pub struct BottomResizeDrag {
    pub mouse_start_y: Pixels,
    pub preview_height_start: Pixels,
}

pub struct DragPreview;

impl Render for DragPreview {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}
