//! Various GUI parameters for runtime control.

use crate::conf::Config;
use crate::fps::Fps;
use crate::serial;
use crate::vis;
use nannou::prelude::*;
use nannou::ui::conrod_core::widget_ids;
use nannou::ui::prelude::*;

pub const COLUMN_W: Scalar = 240.0;
pub const DEFAULT_WIDGET_H: Scalar = 30.0;
pub const PAD: Scalar = 20.0;
pub const WINDOW_WIDTH: u32 = (COLUMN_W + PAD * 2.0) as u32;
pub const WINDOW_HEIGHT: u32 = 770;

widget_ids! {
    pub struct Ids {
        background,
        title_text,
        fullscreen_on_startup_toggle,
        serial_on_startup_toggle,
        serial_on_toggle,
        clear_frame_button,
        random_frame_button,
        vis_fps_text,
        vis_fps_avg_text,
        vis_fps_min_text,
        vis_fps_max_text,
        serial_fps_text,
        serial_fps_avg_text,
        serial_fps_min_text,
        serial_fps_max_text,
        colouration_text,
        hue_slider,
        saturation_slider,
        brightness_slider,
        alpha_slider,
        sustain_slider,
        serial_port_info_text,
    }
}

/// Update the user interface.
pub fn update(
    ref mut ui: UiCell,
    ids: &Ids,
    config: &mut Config,
    serial_on: &mut bool,
    vis_fps: &Fps,
    serial_handle: Option<&serial::Handle>,
    frame: &mut vis::Cbm8032Frame,
) {
    widget::Canvas::new()
        .border(0.0)
        .rgb(0.1, 0.1, 0.1)
        .pad(PAD)
        .set(ids.background, ui);

    // Title

    text("8032 PROJECT")
        .mid_top_of(ids.background)
        .set(ids.title_text, ui);

    // On startup

    for _click in button()
        .mid_left_of(ids.background)
        .down(PAD * 1.5)
        .label(if config.on_startup.fullscreen {
            "Fullscreen On Startup - ENABLED"
        } else {
            "Fullscreen On Startup - DISABLED"
        })
        .color(if config.on_startup.fullscreen {
            color::DARK_BLUE
        } else {
            color::BLACK
        })
        .set(ids.fullscreen_on_startup_toggle, ui)
    {
        config.on_startup.fullscreen = !config.on_startup.fullscreen;
    }

    for _click in button()
        .mid_left_of(ids.background)
        .down(PAD * 0.5)
        .label(if config.on_startup.serial {
            "Serial On Startup - ENABLED"
        } else {
            "Serial On Startup - DISABLED"
        })
        .color(if config.on_startup.serial {
            color::DARK_BLUE
        } else {
            color::BLACK
        })
        .set(ids.serial_on_startup_toggle, ui)
    {
        config.on_startup.serial = !config.on_startup.serial;
    }

    for _click in button()
        .mid_left_of(ids.background)
        .down(PAD * 0.5)
        .label(if *serial_on {
            "Serial - ENABLED"
        } else {
            "Serial - DISABLED"
        })
        .color(if *serial_on {
            color::DARK_BLUE
        } else {
            color::BLACK
        })
        .set(ids.serial_on_toggle, ui)
    {
        *serial_on = !*serial_on;
    }

    let frame_button_w = (COLUMN_W - PAD * 0.5) / 2.0;
    for _click in button()
        .mid_left_of(ids.background)
        .down(PAD * 0.5)
        .w(frame_button_w)
        .label("CLEAR FRAME")
        .set(ids.clear_frame_button, ui)
    {
        *frame = vis::Cbm8032Frame::blank_graphics();
    }

    for _click in button()
        .right(PAD * 0.5)
        .w(frame_button_w)
        .label("RANDOM FRAME")
        .set(ids.random_frame_button, ui)
    {
        vis::randomise_frame_data(&mut frame.data);
    }

    // Vis FPS

    fn fps_to_rgb(fps: f64) -> (f32, f32, f32) {
        let r = clamp(map_range(fps, 0.0, 60.0, 1.0, 0.0), 0.0, 1.0);
        let g = clamp(map_range(fps, 0.0, 60.0, 0.0, 1.0), 0.0, 1.0);
        let b = 0.5;
        (r, g, b)
    }

    widget::Text::new("Visual Rate")
        .mid_left_of(ids.background)
        .down(PAD * 1.5)
        .font_size(16)
        .color(color::WHITE)
        .set(ids.vis_fps_text, ui);

    let label = format!("{:.2} AVG FPS", vis_fps.avg());
    let (r, g, b) = fps_to_rgb(vis_fps.avg());
    widget::Text::new(&label)
        .down(PAD)
        .font_size(14)
        .rgb(r, g, b)
        .set(ids.vis_fps_avg_text, ui);

    let label = format!("{:.2} MIN FPS", vis_fps.min());
    let (r, g, b) = fps_to_rgb(vis_fps.min());
    widget::Text::new(&label)
        .down(PAD * 0.5)
        .font_size(14)
        .rgb(r, g, b)
        .set(ids.vis_fps_min_text, ui);

    let label = format!("{:.2} MAX FPS", vis_fps.max());
    let (r, g, b) = fps_to_rgb(vis_fps.max());
    widget::Text::new(&label)
        .down(PAD * 0.5)
        .font_size(14)
        .rgb(r, g, b)
        .set(ids.vis_fps_max_text, ui);

    // Serial FPS

    widget::Text::new("Serial Rate")
        .mid_left_with_margin_on(ids.background, COLUMN_W / 2.0)
        .align_top_of(ids.vis_fps_text)
        .font_size(16)
        .color(color::WHITE)
        .set(ids.serial_fps_text, ui);

    let serial_fps = serial_handle.map(|handle| handle.frame_hz()).unwrap_or_default();
    let label = format!("{:.2} AVG FPS", serial_fps.avg);
    let (r, g, b) = fps_to_rgb(serial_fps.avg);
    widget::Text::new(&label)
        .down(PAD)
        .font_size(14)
        .rgb(r, g, b)
        .set(ids.serial_fps_avg_text, ui);

    let label = format!("{:.2} MIN FPS", serial_fps.min);
    let (r, g, b) = fps_to_rgb(serial_fps.min);
    widget::Text::new(&label)
        .down(PAD * 0.5)
        .font_size(14)
        .rgb(r, g, b)
        .set(ids.serial_fps_min_text, ui);

    let label = format!("{:.2} MAX FPS", serial_fps.max);
    let (r, g, b) = fps_to_rgb(serial_fps.max);
    widget::Text::new(&label)
        .down(PAD * 0.5)
        .font_size(14)
        .rgb(r, g, b)
        .set(ids.serial_fps_max_text, ui);

    // Colouration

    text("Colouration")
        .down_from(ids.vis_fps_max_text, PAD * 1.5)
        .align_left_of(ids.vis_fps_max_text)
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
    let label_color = color::Color::Rgba(0.4, 0.4, 0.4, 1.0);
    const HUE_YELLOW: f32 = 0.2;
    const HUE_BLUE: f32 = 0.6;
    let label_hue = map_range(config.colouration.hue, HUE_YELLOW, HUE_BLUE, 0.0, 1.0);
    let label = format!("Hue: {:.3}", label_hue);
    for new_hue in slider(config.colouration.hue, HUE_YELLOW, HUE_BLUE)
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

    let label = format!("Sustain: {:.2}", config.sustain);
    for new_sustain in slider(config.sustain, 0.0, 1.0)
        .color(color)
        .label(&label)
        .label_color(label_color)
        .down(PAD * 0.5)
        .set(ids.sustain_slider, ui)
    {
        config.sustain = new_sustain;
    }

    // Serial port info

    if let Some(handle) = serial_handle {
        let info = handle.port_info();
        let mut s = format!("USB Serial Port:  {:?}\n", info.port_name);
        if let serialport::SerialPortType::UsbPort(ref usb) = info.port_type {
            s.push_str(&format!("    VID:  {}\n    PID:  {}\n", usb.vid, usb.pid));
            if let Some(ref serial_number) = usb.serial_number {
                s.push_str(&format!("    Serial Number:  {}\n", serial_number));
            }
            if let Some(ref manufacturer) = usb.manufacturer {
                s.push_str(&format!("    Manufacturer:  {}\n", manufacturer));
            }
            if let Some(ref product) = usb.product {
                s.push_str(&format!("    Product:  {}\n", product));
            }
        }
        widget::Text::new(&s)
            .down(PAD * 1.5)
            .font_size(14)
            .color(color::WHITE)
            .set(ids.serial_port_info_text, ui);
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
