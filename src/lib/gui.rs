//! Various GUI parameters for runtime control.

use nannou::ui::conrod_core::widget_ids;
use nannou::ui::prelude::*;

pub const COLUMN_W: Scalar = 240.0;
pub const PAD: Scalar = 20.0;
pub const WINDOW_WIDTH: u32 = (COLUMN_W + PAD * 2.0) as u32;
pub const WINDOW_HEIGHT: u32 = 720;

widget_ids! {
    pub struct Ids {
        background,
    }
}

/// Update the user interface.
pub fn update(ref mut ui: UiCell, ids: &Ids) {
    widget::Canvas::new().set(ids.background, ui);
}
