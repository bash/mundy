#[cfg(feature = "color-scheme")]
use crate::ColorScheme;
#[cfg(feature = "contrast")]
use crate::Contrast;
#[cfg(feature = "double-click-interval")]
use crate::DoubleClickInterval;
#[cfg(feature = "reduced-motion")]
use crate::ReducedMotion;
#[cfg(feature = "reduced-transparency")]
use crate::ReducedTransparency;
#[cfg(feature = "accent-color")]
use crate::{AccentColor, Srgba};
use crate::{AvailablePreferences, Interest};
#[cfg(feature = "_winrt")]
use com_thread::ComThreadGuard;
use futures_channel::mpsc;
use futures_lite::{stream, Stream, StreamExt as _};
use hook::{register_windows_hook, WindowsHookGuard};
use pin_project_lite::pin_project;
use std::sync::mpsc as std_mpsc;
use std::thread;
#[cfg(feature = "double-click-interval")]
use std::time::Duration;
#[cfg(feature = "_winrt")]
use windows::Win32::System::Com::COINIT_MULTITHREADED;
#[cfg(feature = "double-click-interval")]
use windows::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime;
use windows::Win32::UI::WindowsAndMessaging::WM_SETTINGCHANGE;
#[cfg(any(feature = "color-scheme", feature = "accent-color"))]
use windows::UI::Color;
#[cfg(feature = "contrast")]
use windows::UI::ViewManagement::AccessibilitySettings;
#[cfg(any(feature = "color-scheme", feature = "accent-color"))]
use windows::UI::ViewManagement::UIColorType;
#[cfg(any(
    feature = "color-scheme",
    feature = "accent-color",
    feature = "reduced-motion",
    feature = "reduced-transparency"
))]
use windows::UI::ViewManagement::UISettings;

#[cfg(feature = "_winrt")]
mod com_thread;
mod hook;
mod main_thread;

pin_project! {
    pub(crate) struct PreferencesStream {
        _shutdown: Shutdown,
        #[pin] inner: stream::Boxed<AvailablePreferences>,
    }
}

struct Shutdown(std_mpsc::Sender<Message>);

impl Drop for Shutdown {
    fn drop(&mut self) {
        let _ = self.0.send(Message::Shutdown);
    }
}

impl Stream for PreferencesStream {
    type Item = AvailablePreferences;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.project().inner.poll_next(cx)
    }
}

pub(crate) fn stream(interest: Interest) -> PreferencesStream {
    let (message_tx, message_rx) = std_mpsc::channel();
    let shutdown = Shutdown(message_tx.clone());
    let (sender, receiver) = mpsc::unbounded();
    thread::Builder::new()
        .name(format!("{} COM thread", env!("CARGO_PKG_NAME")))
        .spawn(move || com_thread(sender, message_tx, message_rx, interest))
        .expect("failed to spawn thread");
    PreferencesStream {
        _shutdown: shutdown,
        inner: receiver.boxed(),
    }
}

#[cfg(all(feature = "log", feature = "_winrt"))]
fn log_init_error(error: windows::core::Error) {
    log::warn!("failed to initialize COM: {error}");
}

#[cfg(all(not(feature = "log"), feature = "_winrt"))]
fn log_init_error(_error: windows::core::Error) {}

fn com_thread(
    sender: mpsc::UnboundedSender<AvailablePreferences>,
    msg_tx: std_mpsc::Sender<Message>,
    msg_rx: std_mpsc::Receiver<Message>,
    interest: Interest,
) {
    #[cfg(feature = "_winrt")]
    let _guard = match ComThreadGuard::new(COINIT_MULTITHREADED) {
        Ok(g) => g,
        Err(error) => {
            log_init_error(error);
            _ = sender.unbounded_send(AvailablePreferences::default());
            return;
        }
    };

    let settings = Settings::new();
    let preferences = read_preferences(&settings, interest);
    _ = sender.unbounded_send(preferences);

    let _hook = register_wm_settingchange_hook(msg_tx);

    while let Ok(message) = msg_rx.recv() {
        match message {
            Message::Shutdown => break,
            Message::WM_SETTINGCHANGE => {
                _ = sender.unbounded_send(read_preferences(&settings, interest));
            }
        }
    }
}

struct Settings {
    #[cfg(any(
        feature = "color-scheme",
        feature = "accent-color",
        feature = "reduced-motion",
        feature = "reduced-transparency"
    ))]
    ui: Option<UISettings>,
    #[cfg(feature = "contrast")]
    accessibility: Option<AccessibilitySettings>,
}

impl Settings {
    fn new() -> Self {
        Self {
            #[cfg(any(
                feature = "color-scheme",
                feature = "accent-color",
                feature = "reduced-motion",
                feature = "reduced-transparency"
            ))]
            ui: UISettings::new().ok(),
            #[cfg(feature = "contrast")]
            accessibility: AccessibilitySettings::new().ok(),
        }
    }
}

fn register_wm_settingchange_hook(tx: std_mpsc::Sender<Message>) -> Option<WindowsHookGuard> {
    let result = register_windows_hook(Box::new(move |data| {
        if data.message == WM_SETTINGCHANGE {
            _ = tx.send(Message::WM_SETTINGCHANGE);
        }
    }));
    match result {
        Ok(guard) => Some(guard),
        #[cfg(feature = "log")]
        Err(error) => {
            log::warn!("failed to register windows hook: {error:?}");
            None
        }
        #[cfg(not(feature = "log"))]
        Err(_) => None,
    }
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Shutdown,
    #[allow(non_camel_case_types)]
    WM_SETTINGCHANGE,
}

fn read_preferences(
    #[cfg_attr(not(feature = "_winrt"), allow(unused_variables))] settings: &Settings,
    interest: Interest,
) -> AvailablePreferences {
    let mut preferences = AvailablePreferences::default();

    #[cfg(feature = "color-scheme")]
    if let Some(ui) = &settings.ui {
        if interest.is(Interest::ColorScheme) {
            preferences.color_scheme = read_color_scheme(ui);
        }
    }

    #[cfg(feature = "contrast")]
    if let Some(accessibility) = &settings.accessibility {
        if interest.is(Interest::Contrast) {
            preferences.contrast = read_contrast(accessibility);
        }
    }

    #[cfg(feature = "accent-color")]
    if let Some(ui) = &settings.ui {
        if interest.is(Interest::AccentColor) {
            preferences.accent_color = read_accent_color(ui);
        }
    }

    #[cfg(feature = "reduced-motion")]
    if let Some(ui) = &settings.ui {
        if interest.is(Interest::ReducedMotion) {
            preferences.reduced_motion = read_reduced_motion(ui);
        }
    }

    #[cfg(feature = "reduced-transparency")]
    if let Some(ui) = &settings.ui {
        if interest.is(Interest::ReducedTransparency) {
            preferences.reduced_transparency = read_reduced_transparency(ui);
        }
    }

    #[cfg(feature = "double-click-interval")]
    if interest.is(Interest::DoubleClickInterval) {
        preferences.double_click_interval = read_double_click_time();
    }

    preferences
}

#[cfg(feature = "_winrt")]
macro_rules! try_settings_result {
    ($result:expr) => {
        match $result {
            Ok(result) => result,
            #[cfg(feature = "log")]
            Err(err) => {
                log::warn!("call to WinRT method failed: {err}");
                return Default::default();
            }
            #[cfg(not(feature = "log"))]
            Err(_) => return Default::default(),
        }
    };
}

#[cfg(feature = "accent-color")]
fn read_accent_color(settings: &UISettings) -> AccentColor {
    fn to_srgba(color: Color) -> Srgba {
        Srgba::from_u8_array([color.R, color.G, color.B, color.A])
    }

    let accent = try_settings_result!(settings.GetColorValue(UIColorType::Accent));
    AccentColor(Some(to_srgba(accent)))
}

// TODO: Windows technically supports "less" and "custom" contrast
// but I'm not sure which API to call.
#[cfg(feature = "contrast")]
fn read_contrast(settings: &AccessibilitySettings) -> Contrast {
    let high_contrast = try_settings_result!(settings.HighContrast());
    if high_contrast {
        Contrast::More
    } else {
        Contrast::NoPreference
    }
}

// This is what's recommended by the official docs:
// <https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-windows-themes>
#[cfg(feature = "color-scheme")]
fn read_color_scheme(settings: &UISettings) -> ColorScheme {
    let foreground = try_settings_result!(settings.GetColorValue(UIColorType::Foreground));

    fn is_color_light(color: &Color) -> bool {
        ((5 * color.G as u16) + (2 * color.R as u16) + color.B as u16) > (8 * 128)
    }

    if is_color_light(&foreground) {
        ColorScheme::Dark
    } else {
        ColorScheme::Light
    }
}

#[cfg(feature = "reduced-motion")]
fn read_reduced_motion(settings: &UISettings) -> ReducedMotion {
    let animations = try_settings_result!(settings.AnimationsEnabled());
    if animations {
        ReducedMotion::NoPreference
    } else {
        ReducedMotion::Reduce
    }
}

#[cfg(feature = "reduced-transparency")]
fn read_reduced_transparency(settings: &UISettings) -> ReducedTransparency {
    let advanced_effects = try_settings_result!(settings.AdvancedEffectsEnabled());
    if advanced_effects {
        ReducedTransparency::NoPreference
    } else {
        ReducedTransparency::Reduce
    }
}

#[cfg(feature = "double-click-interval")]
fn read_double_click_time() -> DoubleClickInterval {
    let millis = unsafe { GetDoubleClickTime() };
    DoubleClickInterval(Some(Duration::from_millis(millis as u64)))
}
