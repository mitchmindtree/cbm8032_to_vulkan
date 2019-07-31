//! Various GUI parameters for runtime control.

use nannou::prelude::*;
use nannou::ui::prelude::*;

pub const COLUMN_W: Scalar = 240.0;
pub const SLIDER_H: Scalar = 30.0;
pub const PAD: Scalar = 20.0;
pub const WINDOW_WIDTH: u32 = (COLUMN_W + PAD * 2.0) as u32;
pub const WINDOW_HEIGHT: u32 = 960;
