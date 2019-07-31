use nannou::prelude::*;

mod gui;

struct Model {
    vis_window: window::Id,
    gui_window: window::Id,
}

/// Start the application.
pub fn run() {
    nannou::app(model).update(update).run();
}

fn model(app: &App) -> Model {
    // The CBM 8032 visualisation window.
    let vis_window = app.new_window()
        .with_title("CBM 8032 VIS")
        .view(vis_view)
        .build()
        .unwrap();

    // The GUI window for controlling the visualisation at runtime.
    let gui_window = app.new_window()
        .with_title("CBM 8032 GUI")
        .with_dimensions(gui::WINDOW_WIDTH, gui::WINDOW_HEIGHT)
        .view(gui_view)
        .build()
        .unwrap();

    Model {
        vis_window,
        gui_window,
    }
}

fn update(_app: &App, _model: &mut Model, _update: Update) {}

fn vis_view(app: &App, model: &Model, frame: &Frame) {
    let draw = app.draw_for_window(model.vis_window).expect("no window for ID");
    draw.ellipse().color(STEELBLUE);
    draw.to_frame(app, &frame).unwrap();
}

fn gui_view(app: &App, model: &Model, frame: &Frame) {
    let draw = app.draw_for_window(model.gui_window).expect("no window for ID");
    draw.background().color(PLUM);
    draw.ellipse().color(STEELBLUE);
    draw.to_frame(app, &frame).unwrap();
}
