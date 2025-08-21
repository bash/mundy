#[cfg(feature = "color-scheme")]
use crate::ColorScheme;
#[cfg(feature = "contrast")]
use crate::Contrast;
#[cfg(feature = "double-click-interval")]
use crate::DoubleClickInterval;
#[cfg(feature = "reduced-motion")]
use crate::ReducedMotion;
#[cfg(feature = "accent-color")]
use crate::{AccentColor, Srgba};

use crate::async_rt::block_on;
use crate::stream_utils::{Left, Right, Scan};
use crate::{AvailablePreferences, Interest};
use cfg_if::cfg_if;
use futures_lite::{stream, FutureExt as _, Stream, StreamExt as _};
use std::time::Duration;
use zbus::{
    proxy::SignalStream,
    zvariant::{OwnedValue, Value},
    Connection, Message, Proxy,
};

#[cfg(feature = "log")]
fn log_dbus_connection_error(err: &zbus::Error) {
    log::warn!("failed to connect to dbus: {err:?}");
}

#[cfg(not(feature = "log"))]
fn log_dbus_connection_error(_err: &zbus::Error) {}

#[cfg(feature = "log")]
fn log_initial_settings_retrieval_error(err: &zbus::Error) {
    log::warn!("error retrieving the initial setting values: {err:?}");
}

#[cfg(not(feature = "log"))]
fn log_initial_settings_retrieval_error(_err: &zbus::Error) {}

#[cfg(feature = "log")]
fn log_message_error(err: &zbus::Error) {
    log::debug!("failed to process incoming dbus message: {err:?}");
}

#[cfg(not(feature = "log"))]
fn log_message_error(_err: &zbus::Error) {}

const APPEARANCE: &str = "org.freedesktop.appearance";
#[cfg(feature = "reduced-motion")]
const GNOME_INTERFACE: &str = "org.gnome.desktop.interface";
#[cfg(feature = "double-click-interval")]
const GNOME_PERIPHERALS_MOUSE: &str = "org.gnome.desktop.peripherals.mouse";
#[cfg(feature = "double-click-interval")]
const DOUBLE_CLICK: &str = "double-click";
#[cfg(feature = "color-scheme")]
const COLOR_SCHEME: &str = "color-scheme";
#[cfg(feature = "contrast")]
const CONTRAST: &str = "contrast";
#[cfg(feature = "accent-color")]
const ACCENT_COLOR: &str = "accent-color";
#[cfg(feature = "reduced-motion")]
const ENABLE_ANIMATIONS: &str = "enable-animations";

pub(crate) type PreferencesStream = stream::Boxed<AvailablePreferences>;

pub(crate) fn stream(interest: Interest) -> PreferencesStream {
    preferences_stream(interest).boxed()
}

pub(crate) fn default_stream() -> PreferencesStream {
    stream::once(AvailablePreferences::default()).boxed()
}

pub(crate) fn once_blocking(interest: Interest, timeout: Duration) -> Option<AvailablePreferences> {
    block_on(stream(interest).next().or(timer(timeout)))
}

cfg_if! {
    if #[cfg(feature = "tokio")] {
        async fn timer<T>(duration: Duration) -> Option<T> {
            tokio::time::sleep(duration).await;
            None
        }
    } else if #[cfg(feature = "async-io")] {
        async fn timer<T>(duration: Duration) -> Option<T> {
            async_io::Timer::after(duration).await;
            None
        }
    }
}

fn preferences_stream(interest: Interest) -> impl Stream<Item = AvailablePreferences> {
    stream::once_future(subscribe(interest)).flat_map(move |(preferences, stream)| {
        let initial_value = stream::once(preferences);
        let stream = stream.map(Left).unwrap_or_else(|| Right(stream::empty()));
        initial_value.chain(changes(interest, preferences, stream))
    })
}

fn changes(
    interest: Interest,
    preferences: AvailablePreferences,
    stream: impl Stream<Item = Message>,
) -> impl Stream<Item = AvailablePreferences> {
    Scan::new(
        stream,
        preferences,
        move |mut preferences, message| async move {
            if let Err(err) = apply_message(interest, &mut preferences, message).await {
                log_message_error(&err);
            }
            Some((preferences, preferences))
        },
    )
}

async fn subscribe(interest: Interest) -> (AvailablePreferences, Option<SignalStream<'static>>) {
    match connect().await {
        Ok(proxy) => {
            let stream = setting_changed(&proxy, interest)
                .await
                .inspect_err(log_dbus_connection_error)
                .ok();
            let preferences = initial_preferences(&proxy, interest)
                .await
                .inspect_err(log_initial_settings_retrieval_error)
                .unwrap_or_default();
            (preferences, stream)
        }
        Err(err) => {
            log_dbus_connection_error(&err);
            Default::default()
        }
    }
}

async fn connect() -> zbus::Result<Proxy<'static>> {
    let connection = Connection::session().await?;
    settings_proxy(&connection).await
}

async fn apply_message(
    interest: Interest,
    preferences: &mut AvailablePreferences,
    message: Message,
) -> Result<(), zbus::Error> {
    let body = message.body();
    let (namespace, key, value): (&str, &str, Value) = body.deserialize()?;
    match (namespace, key) {
        #[cfg(feature = "color-scheme")]
        (APPEARANCE, COLOR_SCHEME) if interest.is(Interest::ColorScheme) => {
            preferences.color_scheme = parse_color_scheme(value);
        }
        #[cfg(feature = "contrast")]
        (APPEARANCE, CONTRAST) if interest.is(Interest::Contrast) => {
            preferences.contrast = parse_contrast(value);
        }
        #[cfg(feature = "reduced-motion")]
        (GNOME_INTERFACE, ENABLE_ANIMATIONS) if interest.is(Interest::ReducedMotion) => {
            preferences.reduced_motion = parse_enable_animation(value);
        }
        #[cfg(feature = "accent-color")]
        (APPEARANCE, ACCENT_COLOR) if interest.is(Interest::AccentColor) => {
            preferences.accent_color = parse_accent_color(value);
        }
        #[cfg(feature = "double-click-interval")]
        (GNOME_PERIPHERALS_MOUSE, DOUBLE_CLICK) if interest.is(Interest::DoubleClickInterval) => {
            preferences.double_click_interval = parse_double_click(value);
        }
        _ => {}
    }
    Ok(())
}

async fn initial_preferences(
    proxy: &Proxy<'_>,
    interest: Interest,
) -> zbus::Result<AvailablePreferences> {
    let mut preferences = AvailablePreferences::default();
    #[cfg(feature = "color-scheme")]
    if interest.is(Interest::ColorScheme) {
        preferences.color_scheme = read_setting(proxy, APPEARANCE, COLOR_SCHEME)
            .await
            .map(parse_color_scheme)
            .unwrap_or_default();
    }
    #[cfg(feature = "contrast")]
    if interest.is(Interest::Contrast) {
        preferences.contrast = read_setting(proxy, APPEARANCE, CONTRAST)
            .await
            .map(parse_contrast)
            .unwrap_or_default();
    }
    #[cfg(feature = "reduced-motion")]
    if interest.is(Interest::ReducedMotion) {
        // Ideally this would be something that xdg-desktop-portal gives us.
        preferences.reduced_motion = read_setting(proxy, GNOME_INTERFACE, ENABLE_ANIMATIONS)
            .await
            .map(parse_enable_animation)
            .unwrap_or_default();
    }
    #[cfg(feature = "accent-color")]
    if interest.is(Interest::AccentColor) {
        preferences.accent_color = read_setting(proxy, APPEARANCE, ACCENT_COLOR)
            .await
            .map(parse_accent_color)
            .unwrap_or_default();
    }
    #[cfg(feature = "double-click-interval")]
    if interest.is(Interest::DoubleClickInterval) {
        preferences.double_click_interval =
            read_setting(proxy, GNOME_PERIPHERALS_MOUSE, DOUBLE_CLICK)
                .await
                .map(parse_double_click)
                .unwrap_or_default();
    }
    Ok(preferences)
}

async fn settings_proxy<'a>(connection: &Connection) -> zbus::Result<Proxy<'a>> {
    Proxy::new(
        connection,
        "org.freedesktop.portal.Desktop",
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.Settings",
    )
    .await
}

async fn read_setting(proxy: &Proxy<'_>, namespace: &str, key: &str) -> Option<Value<'static>> {
    proxy
        .call::<_, _, OwnedValue>("Read", &(namespace, key))
        .await
        .ok()
        .map(Value::from)
        .map(flatten_value)
}

// `Read` returns a variant inside a variant *sigh*.
// In theory there's `ReadOne` which fixes this but I
// haven't checked how ubiquitously it is available.
fn flatten_value(value: Value<'_>) -> Value<'_> {
    if let Value::Value(inner) = value {
        *inner
    } else {
        value
    }
}

async fn setting_changed(
    proxy: &Proxy<'_>,
    interest: Interest,
) -> zbus::Result<SignalStream<'static>> {
    proxy
        .receive_signal_with_args("SettingChanged", signal_filter(interest))
        .await
}

fn signal_filter(
    #[cfg_attr(not(feature = "_gnome_only"), expect(unused_variables))] interest: Interest,
) -> &'static [(u8, &'static str)] {
    #[cfg(feature = "_gnome_only")]
    if interest.is(Interest::GnomeOnly) {
        return &[];
    }
    &[(0, APPEARANCE)]
}

/// See <https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.portal.Settings.html>.
#[cfg(feature = "color-scheme")]
fn parse_color_scheme(value: Value) -> ColorScheme {
    match u32::try_from(value) {
        // > `1`: Prefer dark appearance
        Ok(1) => ColorScheme::Dark,
        // > `2`: Prefer light appearance
        Ok(2) => ColorScheme::Light,
        // > `0`: No preference
        // > Unknown values should be treated as `0` (no preference).
        Ok(0) | Ok(_) | Err(_) => ColorScheme::NoPreference,
    }
}

#[cfg(feature = "contrast")]
fn parse_contrast(value: Value) -> Contrast {
    match u32::try_from(value) {
        // > `1`: Higher contrast
        Ok(1) => Contrast::More,
        // > `0`: No preference (normal contrast)
        // > Unknown values should be treated as `0` (no preference).
        Ok(0) | Ok(_) | Err(_) => Contrast::NoPreference,
    }
}

// > Indicates the system’s preferred accent color as a tuple of RGB values in the sRGB color space,
// > in the range [0,1]. Out-of-range RGB values should be treated as an unset accent color.
#[cfg(feature = "accent-color")]
fn parse_accent_color(value: Value) -> AccentColor {
    if let Ok((red, green, blue)) = value.downcast() {
        AccentColor(Some(Srgba {
            red,
            green,
            blue,
            alpha: 1.0,
        }))
    } else {
        AccentColor(None)
    }
}

#[cfg(feature = "reduced-motion")]
fn parse_enable_animation(value: Value) -> ReducedMotion {
    match bool::try_from(value) {
        Ok(false) => ReducedMotion::Reduce,
        Ok(true) | Err(_) => ReducedMotion::NoPreference,
    }
}

// Stored as integer (milliseconds):
// https://gitlab.gnome.org/GNOME/gsettings-desktop-schemas/-/blob/6ad9aaea4dc2929770f2fdf9112280aa5081b6de/schemas/org.gnome.desktop.peripherals.gschema.xml.in#L139
#[cfg(feature = "double-click-interval")]
fn parse_double_click(value: Value) -> DoubleClickInterval {
    // We can't directly convert value to u64 because the underlying value is an i32.
    let value = i32::try_from(value)
        .ok()
        .and_then(|v| u64::try_from(v).ok())
        .map(Duration::from_millis);
    DoubleClickInterval(value)
}
