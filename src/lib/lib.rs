use crate::conf::Config;
use crate::fps::Fps;
use crate::vis::Vis;
use nannou::prelude::*;
use nannou::Ui;

mod conf;
mod fps;
mod gui;
mod serial;
mod vis;

const WINDOW_PAD: i32 = 20;
const GUI_WINDOW_X: i32 = WINDOW_PAD;
const GUI_WINDOW_Y: i32 = WINDOW_PAD;
const VIS_WINDOW_X: i32 = GUI_WINDOW_X + gui::WINDOW_WIDTH as i32 + WINDOW_PAD;
const VIS_WINDOW_Y: i32 = GUI_WINDOW_Y;
const VIS_WINDOW_W: u32 = 960;
const VIS_WINDOW_H: u32 = 540;

struct Model {
    _vis_window: window::Id,
    _gui_window: window::Id,
    config: Config,
    ui: Ui,
    ids: gui::Ids,
    vis: Vis,
    serial_on: bool,
    serial_handle: Option<serial::Handle>,
    last_serial_connection_attempt: Option<std::time::Instant>,
    vis_frame: vis::Cbm8032Frame,
    vis_fps: Fps,
}

/// Start the application.
pub fn run() {
    nannou::app(model).update(update).exit(exit).run();
}

fn model(app: &App) -> Model {
    let assets = app
        .assets_path()
        .expect("failed to find project `assets` directory");

    let config_path = conf::path(&assets);
    let config: Config = load_from_json(config_path)
        .ok()
        .unwrap_or_else(Config::default);

    let vis_window = app
        .new_window()
        .title("CBM 8032 VIS")
        .size(VIS_WINDOW_W, VIS_WINDOW_H)
        .view(vis_view)
        .build()
        .expect("failed to build visualisation window");

    let gui_window = app
        .new_window()
        .title("CBM 8032 GUI")
        .size(gui::WINDOW_WIDTH, gui::WINDOW_HEIGHT)
        .view(gui_view)
        .build()
        .expect("failed to build GUI window");

    app.window(gui_window)
        .expect("GUI window closed unexpectedly")
        .set_outer_position_pixels(GUI_WINDOW_X, GUI_WINDOW_Y);

    {
        let w = app.window(vis_window)
            .expect("visualisation window closed unexpectedly");
        w.set_outer_position_pixels(VIS_WINDOW_X, VIS_WINDOW_Y);
        w.set_cursor_visible(false);
        if config.on_startup.fullscreen {
            w.set_fullscreen(true);
        }
    }

    let serial_on = config.on_startup.serial;
    let serial_handle = None;

    let mut ui = app
        .new_ui()
        .window(gui_window)
        .build()
        .expect("failed to build `Ui` for GUI window");
    let ids = gui::Ids::new(ui.widget_id_generator());

    let window = app.window(vis_window).unwrap();
    let msaa_samples = window.msaa_samples();
    let vis = vis::init(&assets, &*window, msaa_samples);
    let vis_frame = vis::Cbm8032Frame::blank_graphics();
    let vis_fps = Fps::default();
    let last_serial_connection_attempt = None;

    Model {
        _vis_window: vis_window,
        _gui_window: gui_window,
        config,
        ui,
        ids,
        vis,
        serial_on,
        serial_handle,
        last_serial_connection_attempt,
        vis_frame,
        vis_fps,
    }
}

fn update(_app: &App, model: &mut Model, _update: Update) {
    let ui = model.ui.set_widgets();
    let handle = model.serial_handle.as_ref();
    gui::update(
        ui,
        &model.ids,
        &mut model.config,
        &mut model.serial_on,
        &model.vis_fps,
        handle,
        &mut model.vis_frame,
    );

    // If `serial_on` is indicated but we have no stream, start one.
    if model.serial_on && model.serial_handle.is_none() {
        let now = std::time::Instant::now();
        let should_attempt = match model.last_serial_connection_attempt {
            None => true,
            Some(last) => now.duration_since(last) > std::time::Duration::from_secs(1),
        };
        if should_attempt {
            model.last_serial_connection_attempt = Some(now);
            match serial::spawn() {
                Ok(handle) => model.serial_handle = Some(handle),
                Err(err) => eprintln!("failed to start serial stream: {}", err),
            }
        }

    // If `serial_on` is `false` and we have a stream, close the stream.
    } else if !model.serial_on && model.serial_handle.is_some() {
        model.serial_handle.take().unwrap().close();
    }

    // If we have a serial handle and it has closed, drop the handle.
    if model.serial_handle.as_ref().map(|h| h.is_closed()).unwrap_or(true) {
        model.serial_handle.take();
    }

    if let Some(handle) = model.serial_handle.as_ref() {
        if let Some(new_frame) = handle.try_recv_frame() {
            model.vis_frame = new_frame;
        }
    }
}

fn vis_view(_app: &App, model: &Model, frame: Frame) {
    if frame.nth() == 0 {
        frame.clear(BLACK);
    }
    model.vis_fps.sample();
    vis::view(&model.config, &model.vis, &model.vis_frame, frame);
}

fn gui_view(app: &App, model: &Model, frame: Frame) {
    model
        .ui
        .draw_to_frame(app, &frame)
        .expect("failed to draw `Ui` to `Frame`");
}

fn exit(app: &App, mut model: Model) {
    let assets = app
        .assets_path()
        .expect("failed to find project `assets` directory");
    let config_path = conf::path(&assets);
    save_to_json(config_path, &model.config).expect("failed to save config");
    model.serial_handle.take().map(|handle| handle.close());
}
