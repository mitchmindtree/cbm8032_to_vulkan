//! Various GUI parameters for runtime control.

use crate::conf::Config;
use crate::fps::Fps;
use nannou::prelude::*;
use nannou::ui::conrod_core::widget_ids;
use nannou::ui::prelude::*;

pub const COLUMN_W: Scalar = 240.0;
pub const DEFAULT_WIDGET_H: Scalar = 30.0;
pub const PAD: Scalar = 20.0;
pub const WINDOW_WIDTH: u32 = (COLUMN_W + PAD * 2.0) as u32;
pub const WINDOW_HEIGHT: u32 = 720;

widget_ids! {
    pub struct Ids {
        background,
        title_text,
        fps_avg_text,
        fps_min_text,
        fps_max_text,
        fullscreen_on_startup_toggle,
        colouration_text,
        hue_slider,
        saturation_slider,
        brightness_slider,
        alpha_slider,
    }
}

/// Update the user interface.
pub fn update(ref mut ui: UiCell, ids: &Ids, config: &mut Config, vis_fps: &Fps) {
    widget::Canvas::new()
        .border(0.0)
        .rgb(0.1, 0.1, 0.1)
        .pad(PAD)
        .set(ids.background, ui);

    // Title

    text("8032 PROJECT")
        .mid_top_of(ids.background)
        .set(ids.title_text, ui);

    // Fullscreen on startup

    for _click in button()
        .mid_left_of(ids.background)
        .down(PAD * 1.5)
        .label(if config.fullscreen_on_startup {
            "Fullscreen On Startup - ENABLED"
        } else {
            "Fullscreen On Startup - DISABLED"
        })
        .color(if config.fullscreen_on_startup {
            color::DARK_BLUE
        } else {
            color::BLACK
        })
        .set(ids.fullscreen_on_startup_toggle, ui)
    {
        config.fullscreen_on_startup = !config.fullscreen_on_startup;
    }

    // FPS

    fn fps_to_rgb(fps: f64) -> (f32, f32, f32) {
        let r = clamp(map_range(fps, 0.0, 60.0, 1.0, 0.0), 0.0, 1.0);
        let g = clamp(map_range(fps, 0.0, 60.0, 0.0, 1.0), 0.0, 1.0);
        let b = 0.5;
        (r, g, b)
    }

    let label = format!("{:.2} AVG FPS", vis_fps.avg());
    let (r, g, b) = fps_to_rgb(vis_fps.avg());
    widget::Text::new(&label)
        .mid_left_of(ids.background)
        .down(PAD * 1.5)
        .font_size(14)
        .rgb(r, g, b)
        .set(ids.fps_avg_text, ui);

    let label = format!("{:.2} MIN FPS", vis_fps.min());
    let (r, g, b) = fps_to_rgb(vis_fps.min());
    widget::Text::new(&label)
        .down(PAD * 0.5)
        .font_size(14)
        .rgb(r, g, b)
        .set(ids.fps_min_text, ui);

    let label = format!("{:.2} MAX FPS", vis_fps.max());
    let (r, g, b) = fps_to_rgb(vis_fps.max());
    widget::Text::new(&label)
        .down(PAD * 0.5)
        .font_size(14)
        .rgb(r, g, b)
        .set(ids.fps_max_text, ui);

    // Colouration

    text("Colouration")
        .down(PAD * 1.5)
        .font_size(16)
        .set(ids.colouration_text, ui);

    let hsv = hsv(
        config.colouration.hue,
        config.colouration.saturation,
        config.colouration.brightness,
    );
    let lin_srgb: LinSrgb = hsv.into();
    let srgb = Srgb::from_linear(lin_srgb);
    let color = color::Color::Rgba(srgb.red, srgb.green, srgb.blue, 1.0);
    let label_color = color.plain_contrast();
    let label = format!("Hue: {:.2}", config.colouration.hue);
    for new_hue in slider(config.colouration.hue, 0.0, 1.0)
        .color(color)
        .down(PAD)
        .label(&label)
        .label_color(label_color)
        .set(ids.hue_slider, ui)
    {
        config.colouration.hue = new_hue;
    }

    let label = format!("Saturation: {:.2}", config.colouration.saturation);
    for new_saturation in slider(config.colouration.saturation, 0.0, 1.0)
        .color(color)
        .label(&label)
        .label_color(label_color)
        .down(PAD * 0.5)
        .set(ids.saturation_slider, ui)
    {
        config.colouration.saturation = new_saturation;
    }

    let label = format!("Brightness: {:.2}", config.colouration.brightness);
    for new_brightness in slider(config.colouration.brightness, 0.0, 1.0)
        .color(color)
        .label(&label)
        .label_color(label_color)
        .down(PAD * 0.5)
        .set(ids.brightness_slider, ui)
    {
        config.colouration.brightness = new_brightness;
    }

    let label = format!("Alpha: {:.2}", config.colouration.alpha);
    for new_alpha in slider(config.colouration.alpha, 0.0, 1.0)
        .color(color)
        .label(&label)
        .label_color(label_color)
        .down(PAD * 0.5)
        .set(ids.alpha_slider, ui)
    {
        config.colouration.alpha = new_alpha;
    }
}

fn text(s: &str) -> widget::Text {
    widget::Text::new(s).color(color::WHITE)
}

fn button() -> widget::Button<'static, widget::button::Flat> {
    widget::Button::new()
        .w_h(COLUMN_W, DEFAULT_WIDGET_H)
        .label_font_size(12)
        .color(color::DARK_CHARCOAL)
        .label_color(color::WHITE)
        .border(0.0)
}

fn slider(val: f32, min: f32, max: f32) -> widget::Slider<'static, f32> {
    widget::Slider::new(val, min, max)
        .w_h(COLUMN_W, DEFAULT_WIDGET_H)
        .label_font_size(12)
        .color(color::DARK_CHARCOAL)
        .label_color(color::WHITE)
        .border(0.0)
}
