use crate::conf::Config;
use crate::fps::Fps;
use crate::vis::Vis;
use nannou::prelude::*;
use nannou::Ui;

mod conf;
mod fps;
mod gui;
mod vis;

const WINDOW_PAD: i32 = 20;
const GUI_WINDOW_X: i32 = WINDOW_PAD;
const GUI_WINDOW_Y: i32 = WINDOW_PAD;
const VIS_WINDOW_X: i32 = GUI_WINDOW_X + gui::WINDOW_WIDTH as i32 + WINDOW_PAD;
const VIS_WINDOW_Y: i32 = GUI_WINDOW_Y;
const VIS_WINDOW_W: u32 = 1280;
const VIS_WINDOW_H: u32 = 720;

struct Model {
    config: Config,
    _vis_window: window::Id,
    _gui_window: window::Id,
    ui: Ui,
    ids: gui::Ids,
    vis: Vis,
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

    let physical_device = vis::best_gpu(app).expect("no available GPU detected on system");

    let vis_window = app
        .new_window()
        .with_title("CBM 8032 VIS")
        .vk_physical_device(physical_device)
        .with_dimensions(VIS_WINDOW_W, VIS_WINDOW_H)
        .view(vis_view)
        .build()
        .expect("failed to build visualisation window");

    let gui_window = app
        .new_window()
        .with_title("CBM 8032 GUI")
        .with_dimensions(gui::WINDOW_WIDTH, gui::WINDOW_HEIGHT)
        .view(gui_view)
        .build()
        .expect("failed to build GUI window");

    app.window(gui_window)
        .expect("GUI window closed unexpectedly")
        .set_position(GUI_WINDOW_X, GUI_WINDOW_Y);

    app.window(vis_window)
        .expect("visualisation window closed unexpectedly")
        .set_position(VIS_WINDOW_X, VIS_WINDOW_Y);

    if config.fullscreen_on_startup {
        let window = app
            .window(vis_window)
            .expect("visualisation window closed unexpectedly");
        window.set_fullscreen(Some(window.current_monitor()));
        window.hide_cursor(true);
    }

    let mut ui = app
        .new_ui()
        .window(gui_window)
        .build()
        .expect("failed to build `Ui` for GUI window");
    let ids = gui::Ids::new(ui.widget_id_generator());

    let queue = app.window(vis_window).unwrap().swapchain_queue().clone();
    let msaa_samples = app.window(vis_window).unwrap().msaa_samples();
    let vis = vis::init(&assets, queue, msaa_samples);

    let vis_fps = Fps::default();

    Model {
        config,
        _vis_window: vis_window,
        _gui_window: gui_window,
        ui,
        ids,
        vis,
        vis_fps,
    }
}

fn update(_app: &App, model: &mut Model, _update: Update) {
    let ui = model.ui.set_widgets();
    gui::update(ui, &model.ids, &mut model.config, &model.vis_fps);
}

fn vis_view(_app: &App, model: &Model, frame: &Frame) {
    if frame.nth() == 0 {
        frame.clear(BLACK);
    }

    model.vis_fps.sample();

    vis::view(&model.config, &model.vis, frame);
}

fn gui_view(app: &App, model: &Model, frame: &Frame) {
    model
        .ui
        .draw_to_frame(app, frame)
        .expect("failed to draw `Ui` to `Frame`");
}

fn exit(app: &App, model: Model) {
    let assets = app
        .assets_path()
        .expect("failed to find project `assets` directory");
    let config_path = conf::path(&assets);
    save_to_json(config_path, &model.config).expect("failed to save config");
}
