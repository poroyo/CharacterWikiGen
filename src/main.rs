mod panels;
use panels::BigFrame;

#[cfg(not(target_arch = "wasm32"))]
use eframe::{epaint::vec2, run_native};


#[cfg(not(target_arch = "wasm32"))]
fn main() {
    let native_options = eframe::NativeOptions {
        initial_window_size: Some(vec2(1200., 1000.)),
        min_window_size: Some(vec2(400.0, 400.0)),
        resizable: true,
        drag_and_drop_support: true,
        default_theme: eframe::Theme::Dark,
        ..Default::default()
    };

    run_native(
        "TestApp",
        native_options,
        Box::new(|cc| Box::new(BigFrame::_new(cc))),
    )
    .expect("Failed to load the App");
}

#[cfg(target_arch = "wasm32")]
fn main() {
    let web_options = eframe::WebOptions::default();
    tracing_wasm::set_as_global_default();
    console_error_panic_hook::set_once();

    wasm_bindgen_futures::spawn_local(async {
        eframe::start_web(
            "the_canvas_id", // hardcode it
            web_options,
            Box::new(|cc| Box::new(BigFrame::_new(cc)))
        )
        .await
        .expect("failed to start eframe");
    });
}