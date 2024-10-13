#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

use bevy_color::{ColorToPacked, Oklcha, Srgba};
use eframe::egui::{self, style::Selection, Color32, Stroke, Style};
use egui_demo_lib::{View as _, WidgetGallery};
use egui_theme_switch::global_theme_switch;
use mundy::{Interest, Preferences, Subscription};

#[cfg(not(target_arch = "wasm32"))]
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([350.0, 590.0]),
        ..Default::default()
    };
    eframe::run_native(
        "egui example: custom style",
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
    )
}

#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;

    // Redirect `log` message to `console.log` and friends:

    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let web_options = eframe::WebOptions::default();

    wasm_bindgen_futures::spawn_local(async {
        let canvas = web_sys::window()
            .unwrap()
            .document()
            .unwrap()
            .get_element_by_id("egui-app")
            .unwrap();
        let start_result = eframe::WebRunner::new()
            .start(
                canvas.dyn_into().unwrap(),
                web_options,
                Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
            )
            .await;

        // Remove the loading text and spinner:
        let loading_text = web_sys::window()
            .and_then(|w| w.document())
            .and_then(|d| d.get_element_by_id("loading_text"));
        if let Some(loading_text) = loading_text {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}

fn use_accent(style: &mut Style, accent: Srgba) {
    let accent = Oklcha::from(accent);
    let hyperlink_lightness = if style.visuals.dark_mode { 0.7 } else { 0.5 };
    let cursor_lightness = if style.visuals.dark_mode { 0.9 } else { 0.4 };
    let sel_stroke = if style.visuals.dark_mode {
        Color32::WHITE
    } else {
        Color32::BLACK
    };
    let sel_fill_lightness = if style.visuals.dark_mode { 0.3 } else { 0.9 };

    style.visuals.hyperlink_color = to_epaint(accent.with_lightness(hyperlink_lightness));
    style.visuals.text_cursor.stroke.color = to_epaint(accent.with_lightness(cursor_lightness));
    style.visuals.selection = Selection {
        bg_fill: to_epaint(accent.with_lightness(sel_fill_lightness)),
        stroke: Stroke {
            color: sel_stroke,
            ..style.visuals.selection.stroke
        },
    };
}

fn to_epaint(color: impl Into<Srgba>) -> Color32 {
    let color = color.into().to_u8_array();
    Color32::from_rgba_premultiplied(color[0], color[1], color[2], color[3])
}

struct MyApp {
    widget_gallery: WidgetGallery,
    _subscription: Subscription,
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx); // Needed for the "Widget Gallery" demo
        let subscription = Preferences::subscribe(Interest::All, update_style(cc.egui_ctx.clone()));
        Self {
            widget_gallery: WidgetGallery::default(),
            _subscription: subscription,
        }
    }
}

fn update_style(ctx: egui::Context) -> impl Fn(Preferences) {
    move |preferences| {
        if let Some(accent) = preferences.accent_color.0 {
            ctx.all_styles_mut(|style| use_accent(style, accent.into()));
            ctx.request_repaint();
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("egui using a customized style");
            ui.label("Switch between dark and light mode to see the different styles in action.");
            global_theme_switch(ui);
            ui.separator();
            self.widget_gallery.ui(ui);
        });
    }
}
