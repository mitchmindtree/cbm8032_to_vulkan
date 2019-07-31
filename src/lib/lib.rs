use crate::vis::Vis;
use nannou::prelude::*;
use nannou::Ui;

mod gui;
mod vis;

struct Model {
    vis_window: window::Id,
    _gui_window: window::Id,
    ui: Ui,
    ids: gui::Ids,
    vis: Vis,
}

/// Start the application.
pub fn run() {
    nannou::app(model).update(update).run();
}

fn model(app: &App) -> Model {
    let physical_device = vis::best_gpu(app).expect("no available GPU detected on system");

    let vis_window = app
        .new_window()
        .with_title("CBM 8032 VIS")
        .vk_physical_device(physical_device)
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

    let mut ui = app
        .new_ui()
        .window(gui_window)
        .build()
        .expect("failed to build `Ui` for GUI window");
    let ids = gui::Ids::new(ui.widget_id_generator());

    let assets = app
        .assets_path()
        .expect("failed to find project `assets` directory");
    let queue = app.window(vis_window).unwrap().swapchain_queue().clone();
    let msaa_samples = app.window(vis_window).unwrap().msaa_samples();
    let vis = vis::init(&assets, queue, msaa_samples);

    Model {
        vis_window,
        _gui_window: gui_window,
        ui,
        ids,
        vis,
    }
}

fn update(_app: &App, model: &mut Model, _update: Update) {
    let ui = model.ui.set_widgets();
    gui::update(ui, &model.ids);
}

fn vis_view(app: &App, model: &Model, frame: &Frame) {
    if frame.nth() == 0 {
        frame.clear(BLACK);
    }

    let device = app.window(model.vis_window)
        .expect("visualisation window is inaccessible")
        .swapchain_queue()
        .device()
        .clone();

    vis::view(&model.vis, device, frame);
}

fn gui_view(app: &App, model: &Model, frame: &Frame) {
    model
        .ui
        .draw_to_frame(app, frame)
        .expect("failed to draw `Ui` to `Frame`");
}
