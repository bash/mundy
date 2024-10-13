// For the safety review: https://github.com/madsmtm/objc2/tree/master/crates/header-translator#what-is-required-for-a-method-to-be-safe

use crate::stream_utils::Scan;
#[cfg(feature = "color-scheme")]
use crate::ColorScheme;
#[cfg(feature = "contrast")]
use crate::Contrast;
#[cfg(feature = "reduced-motion")]
use crate::ReducedMotion;
#[cfg(feature = "reduced-transparency")]
use crate::ReducedTransparency;
#[cfg(feature = "accent-color")]
use crate::{AccentColor, Srgba};
use crate::{AvailablePreferences, Interest};
use futures_channel::mpsc;
use futures_core::stream::BoxStream;
use futures_core::Stream;
use futures_util::{future, stream, StreamExt as _};
#[cfg(feature = "_macos-accessibility")]
use objc2::rc::Retained;
use objc2_app_kit::NSApplication;
#[cfg(feature = "_macos-accessibility")]
use objc2_app_kit::NSWorkspace;
#[cfg(feature = "color-scheme")]
use objc2_app_kit::{NSAppearance, NSAppearanceNameAqua, NSAppearanceNameDarkAqua};
#[cfg(feature = "accent-color")]
use objc2_app_kit::{NSColor, NSColorSpace};
use objc2_foundation::MainThreadMarker;
#[cfg(feature = "color-scheme")]
use objc2_foundation::{NSArray, NSCopying as _};
use observer::{Observer, ObserverRegistration};
use pin_project_lite::pin_project;
use preference::Preference;

#[cfg(feature = "color-scheme")]
mod main_thread;
mod observer;
mod preference;

pub(crate) fn stream(interest: Interest) -> PreferencesStream {
    let mtm =
        MainThreadMarker::new().expect("on macOS, `subscribe` must be called from the main thread");
    let (sender, receiver) = mpsc::unbounded();

    // A lot of APIs (including the notification center) only proper work when
    // NSApplication.shared is initialized.
    let application = NSApplication::sharedApplication(mtm);
    let observer = Observer::register(&application, sender, interest);
    let initial_value = get_preferences(interest, &application);

    PreferencesStream {
        inner: stream::once(future::ready(initial_value))
            .chain(changes(initial_value, receiver))
            .boxed(),
        _observer: observer,
    }
}

pin_project! {
    pub(crate) struct PreferencesStream {
        #[pin] inner: BoxStream<'static, AvailablePreferences>,
        _observer: ObserverRegistration,
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

fn changes(
    seed: AvailablePreferences,
    receiver: mpsc::UnboundedReceiver<Preference>,
) -> impl Stream<Item = AvailablePreferences> {
    Scan::new(receiver, seed, |prefs, pref| async move {
        let updated = pref.apply(prefs);
        Some((updated, updated))
    })
}

fn get_preferences(
    interest: Interest,
    #[cfg_attr(not(feature = "color-scheme"), expect(unused_variables))]
    application: &NSApplication,
) -> AvailablePreferences {
    let mut preferences = AvailablePreferences::default();

    #[cfg(feature = "color-scheme")]
    if interest.is(Interest::ColorScheme) {
        preferences.color_scheme = to_color_scheme(&application.effectiveAppearance());
    }

    #[cfg(feature = "contrast")]
    if interest.is(Interest::Contrast) {
        preferences.contrast = get_contrast();
    }

    #[cfg(feature = "reduced-motion")]
    if interest.is(Interest::ReducedMotion) {
        preferences.reduced_motion = get_reduced_motion();
    }

    #[cfg(feature = "reduced-transparency")]
    if interest.is(Interest::ReducedTransparency) {
        preferences.reduced_transparency = get_reduced_transparency();
    }

    #[cfg(feature = "accent-color")]
    if interest.is(Interest::AccentColor) {
        preferences.accent_color = get_accent_color();
    }

    preferences
}

#[cfg(feature = "_macos-accessibility")]
fn get_shared_workspace() -> Retained<NSWorkspace> {
    // SAFETY:
    // * `sharedWorkspace` is safe to access from any thread.
    //    Source: <https://developer.apple.com/documentation/appkit/nsworkspace/1530344-sharedworkspace>
    // * Doesn't take any raw pointers.
    // * Has no documented preconditions.
    // * Has no documented exceptions.
    unsafe { NSWorkspace::sharedWorkspace() }
}

#[cfg(feature = "color-scheme")]
fn to_color_scheme(appearance: &NSAppearance) -> ColorScheme {
    let light = unsafe { NSAppearanceNameAqua };
    let dark = unsafe { NSAppearanceNameDarkAqua };
    let names = NSArray::from_id_slice(&[light.copy(), dark.copy()]);

    match appearance.bestMatchFromAppearancesWithNames(&names) {
        Some(best_match) if &*best_match == dark => ColorScheme::Dark,
        Some(_) => ColorScheme::Light,
        None => ColorScheme::NoPreference,
    }
}

#[cfg(feature = "contrast")]
fn get_contrast() -> Contrast {
    let workspace = get_shared_workspace();
    // SAFETY: Similar as for `get_shared_workspace()`.
    let increase_contrast = unsafe { workspace.accessibilityDisplayShouldIncreaseContrast() };
    if increase_contrast {
        Contrast::More
    } else {
        Contrast::NoPreference
    }
}

#[cfg(feature = "reduced-motion")]
fn get_reduced_motion() -> ReducedMotion {
    let workspace = get_shared_workspace();
    // SAFETY: Similar as for `get_shared_workspace()`.
    let reduce_motion = unsafe { workspace.accessibilityDisplayShouldReduceMotion() };
    if reduce_motion {
        ReducedMotion::Reduce
    } else {
        ReducedMotion::NoPreference
    }
}

#[cfg(feature = "reduced-transparency")]
fn get_reduced_transparency() -> ReducedTransparency {
    let workspace = get_shared_workspace();
    // SAFETY: Similar as for `get_shared_workspace()`.
    let reduce_motion = unsafe { workspace.accessibilityDisplayShouldReduceTransparency() };
    if reduce_motion {
        ReducedTransparency::Reduce
    } else {
        ReducedTransparency::NoPreference
    }
}

#[cfg(feature = "accent-color")]
fn get_accent_color() -> AccentColor {
    let color = unsafe { NSColor::controlAccentColor() };
    AccentColor(to_srgb(&color))
}

#[cfg(feature = "accent-color")]
fn to_srgb(color: &NSColor) -> Option<Srgba> {
    let srgb = unsafe { NSColorSpace::sRGBColorSpace() };
    let color_in_srgb = unsafe { color.colorUsingColorSpace(&srgb) }?;
    // We have to cast because on 32-bit platforms, `CGFloat` = f32.
    Some(Srgba {
        red: unsafe { color_in_srgb.redComponent() } as _,
        green: unsafe { color_in_srgb.greenComponent() } as _,
        blue: unsafe { color_in_srgb.blueComponent() } as _,
        alpha: unsafe { color_in_srgb.alphaComponent() } as _,
    })
}
