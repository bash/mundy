#![cfg_attr(docsrs, feature(doc_auto_cfg, doc_cfg))]
#![forbid(
    clippy::dbg_macro,
    clippy::missing_safety_doc,
    clippy::unnecessary_safety_comment,
    clippy::unnecessary_safety_doc,
    unsafe_op_in_unsafe_fn
)]
#![deny(clippy::unwrap_used)]

//! Your friendly neighbourhood ~~whale~~ crate for reading various system-level
//! accessibility and UI preferences across platforms 🐋
//!
//! The following preferences are supported:
//! * [`AccentColor`]—The user's current system wide accent color preference.
//! * [`ColorScheme`]—The user's preference for either light or dark mode.
//! * [`Contrast`]—The user's preferred contrast level.
//! * [`ReducedMotion`]—The user's reduced motion preference.
//! * [`ReducedTransparency`]—The user's reduced transparency preference.
//! * [`DoubleClickInterval`]—The maximum amount of time allowed between the first and second click.
//!
//! Note that each preference has a corresponding [feature flag](`feature_flags`).
//! By turning off [default features](https://doc.rust-lang.org/cargo/reference/features.html#the-default-feature)
//! you will only "pay" for what you actually need.
//!
//! ## Example
//! The easiest way to get the preferences is to use the
//! [`Preferences::stream`] function to create a stream that is continually
//! updated when things change:
//!
//! ```no_run
//! use mundy::{Preferences, Interest};
//! use futures_lite::StreamExt as _;
//!
//! // Interest tells mundy which preferences it should monitor for you.
//! // use `Interest::All` if you're interested in all preferences.
//! let mut stream = Preferences::stream(Interest::AccentColor);
//! # let _ = async move {
//! while let Some(preferences) = stream.next().await {
//!     eprintln!("accent color: {:?}", preferences.accent_color);
//! }
//! # };
//! ```
//!
//! Alternatively, there's [`Preferences::subscribe`] which
//! accepts a simple callback function instead.
//!
//! ## Errors
//! Most errors (except some fatal errors at startup) are simply ignored
//! and the default value for the preference (which is usually `NoPreference`) is returned.
//! It can be useful to turn on the `log` feature to find out what's going on.
//!
//! <br>
//!
//! <small>«*I believe in a universe that doesn't care and people
//! who do. [...] but this whale is pretty cool.* ― Angus</small>

use futures_lite::Stream;
use pin_project_lite::pin_project;
use stream_utils::Dedup;

#[macro_use]
mod impls;
mod interest;
pub use interest::*;
mod async_rt;
#[cfg(feature = "callback")]
mod callback;
#[cfg(feature = "callback")]
pub use callback::*;
#[cfg(feature = "accent-color")]
mod color;
#[cfg(feature = "accent-color")]
pub use color::*;
mod once_blocking;
mod stream_utils;

/// # Feature Flags
///
/// * `epaint`—Enable converting from [`Srgba`] to [`epaint::Color32`].
/// * `bevy_color`—Enable converting from [`Srgba`] to [`bevy_color::Srgba`].
/// * `log`—Enable logging.
/// * `callback`—Enable the synchronous [`Preferences::subscribe`] function (*default*).
/// * `color-scheme`—Enable support for [`ColorScheme`] (*default*).
/// * `contrast`—Enable support for [`Contrast`] (*default*).
/// * `reduced-motion`—Enable support for [`ReducedMotion`] (*default*).
/// * `reduced-transparency`—Enable support for [`ReducedTransparency`] (*default*).
/// * `accent-color`—Enable support for [`AccentColor`] (*default*).
/// * `double-click-interval`—Enable support for [`DoubleClickInterval`] (*default*).
/// * (Linux) `async-io`—Use `zbus` with `async-io` (*default*).
/// * (Linux) `tokio`—Use `zbus` with `tokio` instead of `async-io`.
///
/// ## Turning Off Default Features
///
/// If you turn off [default features](https://doc.rust-lang.org/cargo/reference/features.html#the-default-feature),
/// you will have to pick between one of the two available async runtimes `async-io` and `tokio` to enable.
#[cfg(doc)]
#[cfg_attr(docsrs, doc(cfg(doc)))]
pub mod feature_flags {}

#[cfg(doctest)]
#[doc = include_str!("../readme.md")]
pub mod readme_doctest {}

/// A collection of preferences retrieved by calling either
/// [`Preferences::stream`] or [`Preferences::subscribe`].
///
/// Which fields are filled in is determined by the [`Interest`]
/// you provide when creating a stream or subscription.
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub struct Preferences {
    /// The user's preference for either light or dark mode.
    #[cfg(feature = "color-scheme")]
    pub color_scheme: ColorScheme,
    /// The user's preferred contrast level.
    #[cfg(feature = "contrast")]
    pub contrast: Contrast,
    /// The user's reduced motion preference.
    #[cfg(feature = "reduced-motion")]
    pub reduced_motion: ReducedMotion,
    /// The user's reduced transparency preference.
    #[cfg(feature = "reduced-transparency")]
    pub reduced_transparency: ReducedTransparency,
    /// The user's current system wide accent color preference.
    #[cfg(feature = "accent-color")]
    pub accent_color: AccentColor,
    /// The maximum amount of time that may occur between the first and second click
    /// event for it to count as double click.
    #[cfg(feature = "double-click-interval")]
    pub double_click_interval: DoubleClickInterval,
}

impl Preferences {
    /// Creates a new stream for a selection of system preferences given by `interests`.
    /// Should be called from the main thread *after* setting up an event loop (e.g. using winit).
    ///
    /// The stream is guaranteed to contain at least one item with the initial preferences.
    ///
    /// You can use [`Preferences::subscribe`] if you don't want to manage
    /// spawning an async runtime yourself.
    ///
    #[doc = include_str!("doc/caveats.md")]
    pub fn stream(interest: Interest) -> PreferencesStream {
        // TODO: handle empty interest
        PreferencesStream {
            inner: Dedup::new(imp::stream(interest)),
        }
    }
}

pin_project! {
    /// A stream that continually yields preferences
    /// whenever they are changed. Created by [`Preferences::stream()`].
    pub struct PreferencesStream {
        #[pin] inner: Dedup<imp::PreferencesStream>,
    }
}

#[cfg(test)]
static_assertions::assert_impl_all!(PreferencesStream: Send);

impl Stream for PreferencesStream {
    type Item = Preferences;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.project();
        this.inner.poll_next(cx).map(|o| o.map(Preferences::from))
    }
}

impls! {
    #[cfg(target_os = "linux")]
    mod freedesktop supports {
        "color-scheme" color_scheme,
        "contrast" contrast,
        "reduced-motion" reduced_motion,
        "accent-color" accent_color,
        "double-click-interval" double_click_interval,
    };

    #[cfg(windows)]
    mod windows supports {
        "color-scheme" color_scheme,
        "contrast" contrast,
        "reduced-motion" reduced_motion,
        "accent-color" accent_color,
        "reduced-transparency" reduced_transparency,
        "double-click-interval" double_click_interval,
    };

    #[cfg(target_os = "macos")]
    mod macos supports {
        "color-scheme" color_scheme,
        "contrast" contrast,
        "reduced-motion" reduced_motion,
        "reduced-transparency" reduced_transparency,
        "accent-color" accent_color,
        "double-click-interval" double_click_interval,
    };

    #[cfg(all(target_family = "wasm", target_os = "unknown"))]
    mod web supports {
        "color-scheme" color_scheme,
        "contrast" contrast,
        "reduced-motion" reduced_motion,
        "accent-color" accent_color,
        "reduced-transparency" reduced_transparency,
    };
}

/// The user's preference for either light or dark mode.
///
/// See also <https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-color-scheme>.
///
/// ## Sources
/// * Linux: `org.freedesktop.appearance color-scheme` from the [XDG Settings portal][xdg].
/// * Windows: [`UISettings.GetColorValue(UIColorType::Foreground)`](https://learn.microsoft.com/en-us/windows/apps/desktop/modernize/ui/apply-windows-themes#know-when-dark-mode-is-enabled)
/// * macOS: `NSApplication.effectiveAppearance`
/// * Web: `@media (prefers-color-scheme: ...)`
///
/// [xdg]: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.impl.portal.Settings.html
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[cfg(feature = "color-scheme")]
pub enum ColorScheme {
    /// Indicates that the user has not expressed an active preference,
    /// that the current platform doesn't support a color scheme preference
    /// or that an error occurred while trying to retrieve the preference.
    #[default]
    NoPreference,
    /// Indicates that the user prefers an interface with a light appearance.
    Light,
    /// Indicates that the user prefers an interface with a dark appearance.
    Dark,
}

/// The user's preferred contrast level.
///
/// See also <https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-contrast>
///
/// ## Sources
/// * Linux: `org.freedesktop.appearance contrast` from the [XDG Settings portal][xdg].
/// * Windows: [`AccessibilitySettings.HighContrast`](https://learn.microsoft.com/en-us/uwp/api/windows.ui.viewmanagement.accessibilitysettings.highcontrast)
/// * macOS: [`accessibilityDisplayShouldIncreaseContrast`](https://developer.apple.com/documentation/appkit/nsworkspace/1526290-accessibilitydisplayshouldincrea)
/// * Web: `@media (prefers-contrast: ...)`
///
/// [xdg]: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.impl.portal.Settings.html
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[cfg(feature = "contrast")]
pub enum Contrast {
    /// Indicates that the user has not expressed an active preference,
    /// that the current platform doesn't support a contrast preference
    /// or that an error occurred while trying to retrieve the preference.
    #[default]
    NoPreference,
    /// Indicates that the user prefers an interface with a higher level of contrast.
    More,
    /// Indicates that the user prefers an interface with a lower level of contrast.
    Less,
    /// Indicates that the user has configured a specific set of colors (forced color mode)
    /// and the contrast from these colors neither matches [`Contrast::More`] or [`Contrast::Less`].
    Custom,
}

/// The user prefers to have a minimal amount
/// of motion. Especially motion that simulates the third dimension.
///
/// Such motion can cause discomfort to people with [vestibular disorders](https://www.a11yproject.com/posts/understanding-vestibular-disorders/).
///
/// See also <https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion>.
///
/// ## Sources
/// * Linux (GNOME-only): `org.gnome.desktop.interface enable-animations` from the [XDG Settings portal][xdg].
/// * Windows: [`UISettings.AnimationsEnabled`](https://learn.microsoft.com/en-us/uwp/api/windows.ui.viewmanagement.uisettings.animationsenabled)
/// * macOS: [`accessibilityDisplayShouldReduceMotion`](https://developer.apple.com/documentation/appkit/nsworkspace/1644069-accessibilitydisplayshouldreduce)
/// * Web: `@media (prefers-reduced-motion: ...)`
///
/// [xdg]: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.impl.portal.Settings.html
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[cfg(feature = "reduced-motion")]
pub enum ReducedMotion {
    /// Indicates that the user has not expressed an active preference,
    /// that the current platform doesn't support a reduced motion preference
    /// or that an error occurred while trying to retrieve the preference.
    #[default]
    NoPreference,
    /// Indicates that the user prefers a minimal amount of motion.
    Reduce,
}

/// Indicates that applications should not use transparent or semitransparent backgrounds.
///
/// See also <https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-transparency>.
///
/// ## Sources
/// * Windows: [`UISettings.AdvancedEffectsEnabled`](https://learn.microsoft.com/en-us/uwp/api/windows.ui.viewmanagement.uisettings.advancedeffectsenabled)
/// * macOS: [`accessibilityDisplayShouldReduceTransparency`](https://developer.apple.com/documentation/appkit/nsworkspace/1533006-accessibilitydisplayshouldreduce)
/// * Web: `@media (prefers-reduced-transparency: ...)`
///
/// [xdg]: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.impl.portal.Settings.html
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[cfg(feature = "reduced-transparency")]
pub enum ReducedTransparency {
    /// Indicates that the user has not expressed an active preference,
    /// that the current platform doesn't support a reduced transparency preference
    /// or that an error occurred while trying to retrieve the preference.
    #[default]
    NoPreference,
    /// Indicates that the user prefers an interface with no transparent
    /// or semitransparent backgrounds.
    Reduce,
}

/// The user's current system wide accent color preference.
///
/// ## Sources
/// * Linux: `org.freedesktop.appearance accent-color` from the [XDG Settings portal][xdg].
/// * Windows: [`UISettings.GetColorValue(UIColorType::Accent)`](https://learn.microsoft.com/en-us/uwp/api/windows.ui.viewmanagement.uisettings)
/// * macOS: [`NSColor.controlAccentColor`](https://developer.apple.com/documentation/appkit/nscolor/3000782-controlaccentcolor)
/// * Web: The [`AccentColor`](https://developer.mozilla.org/en-US/docs/Web/CSS/system-color#accentcolor) system color.
///
/// [xdg]: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.impl.portal.Settings.html
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[cfg(feature = "accent-color")]
pub struct AccentColor(pub Option<Srgba>);

/// The maximum amount of time that may occur between the first and second click
/// event for it to count as double click.
///
/// A typical value for this setting is ~500 ms.
///
/// ## Sources
/// * Linux (GNOME-only): `org.gnome.desktop.peripherals.mouse double-click` from the [XDG Settings portal][xdg].
/// * Windows: [`GetDoubleClickTime`](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getdoubleclicktime)
/// * macOS: [`NSEvent.doubleClickInterval`](https://developer.apple.com/documentation/appkit/nsevent/1528384-doubleclickinterval)
/// * Web: Unsupported
///
/// [xdg]: https://flatpak.github.io/xdg-desktop-portal/docs/doc-org.freedesktop.impl.portal.Settings.html
#[derive(Debug, Default, Clone, Copy, PartialEq)]
#[cfg(feature = "double-click-interval")]
pub struct DoubleClickInterval(pub Option<std::time::Duration>);

// TODO: Windows also has a double click size:
// https://github.com/dotnet/winforms/blob/7376e50c5a762131398992def2e76f4586fe5025/src/System.Windows.Forms/src/System/Windows/Forms/SystemInformation.cs#L263
// https://github.com/dotnet/winforms/blob/7376e50c5a762131398992def2e76f4586fe5025/src/System.Windows.Forms/src/System/Windows/Forms/SystemInformation.cs#L263
