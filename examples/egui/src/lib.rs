use bevy_color::{ColorToComponents as _, ColorToPacked, Oklcha, Srgba};
use eframe::egui::{self, style::Selection, Color32, Stroke, Style};
use egui_demo_lib::{View as _, WidgetGallery};
use egui_theme_switch::global_theme_switch;
use mundy::{Interest, Preferences, Subscription};

pub struct DemoApp {
    widget_gallery: WidgetGallery,
    _subscription: Subscription,
}

impl DemoApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        egui_extras::install_image_loaders(&cc.egui_ctx); // Needed for the "Widget Gallery" demo
        let subscription = Preferences::subscribe(Interest::All, update_style(cc.egui_ctx.clone()));
        Self {
            widget_gallery: WidgetGallery::default(),
            _subscription: subscription,
        }
    }
}

#[cfg(target_os = "android")]
#[no_mangle]
pub fn android_main(app: winit::platform::android::activity::AndroidApp) {
    #[cfg(target_os = "android")]
    android_logger::init_once(
        android_logger::Config::default()
            .with_filter(
                android_logger::FilterBuilder::new()
                    .filter(None, log::LevelFilter::Warn)
                    .filter_module("mundy", log::LevelFilter::Trace)
                    .filter_module("egui_example", log::LevelFilter::Trace)
                    .build(),
            )
            .with_max_level(log::LevelFilter::Trace),
    );
    let options = eframe::NativeOptions {
        android_app: Some(app),
        ..Default::default()
    };
    eframe::run_native(
        "egui example: custom style",
        options,
        Box::new(|cc| Ok(Box::new(DemoApp::new(cc)))),
    )
    .unwrap();
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

fn to_bevy(color: mundy::Srgba) -> Srgba {
    Srgba::from_f32_array(color.to_f64_array().map(|c| c as f32))
}

fn update_style(ctx: egui::Context) -> impl Fn(Preferences) {
    move |preferences| {
        log::info!("got new preferences: {preferences:#?}");
        if let Some(accent) = preferences.accent_color.0 {
            ctx.all_styles_mut(|style| use_accent(style, to_bevy(accent)));
            ctx.request_repaint();
        }
    }
}

impl eframe::App for DemoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Reserve some space at the top so the demo ui isn't hidden behind the android status bar
        // TODO(lucasmerlin): This is a pretty big hack, should be fixed once safe_area implemented
        // for android:
        // https://github.com/rust-windowing/winit/issues/3910
        #[cfg(target_os = "android")]
        egui::TopBottomPanel::top("status_bar_space").show(ctx, |ui| {
            ui.set_height(32.0);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("egui using a customized style");
            ui.label("Switch between dark and light mode to see the different styles in action.");
            global_theme_switch(ui);
            ui.separator();
            self.widget_gallery.ui(ui);
        });
    }
}
