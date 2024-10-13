#[cfg(feature = "accent-color")]
use crate::AccentColor;
#[cfg(feature = "color-scheme")]
use crate::ColorScheme;
#[cfg(feature = "contrast")]
use crate::Contrast;
#[cfg(feature = "reduced-motion")]
use crate::ReducedMotion;
#[cfg(feature = "reduced-transparency")]
use crate::ReducedTransparency;

use crate::stream_utils::Scan;
use crate::{AvailablePreferences, Interest};
use drop_on_main_thread::DropOnMainThread;
use event_listener::EventListenerGuard;
use futures_channel::mpsc;
use futures_lite::{stream, Stream, StreamExt as _};
use pin_project_lite::pin_project;
use web_sys::window;

#[cfg(feature = "accent-color")]
mod accent_color;
#[cfg(any(feature = "reduced-motion", feature = "reduced-transparency"))]
mod bool;
mod event_listener;
#[cfg(any(feature = "contrast", feature = "color-scheme"))]
#[macro_use]
mod multi_value;
mod drop_on_main_thread;

#[cfg(feature = "accent-color")]
type AccentColorObserver = Option<DropOnMainThread<accent_color::AccentColorObserver>>;

#[cfg(not(feature = "accent-color"))]
type AccentColorObserver = ();

pin_project! {
    pub(crate) struct PreferencesStream {
        _guards: Vec<DropOnMainThread<EventListenerGuard>>,
        _accent_color: AccentColorObserver,
        #[pin] inner: stream::Boxed<AvailablePreferences>,
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
    let Some(window) = window() else {
        #[cfg(feature = "log")]
        log::warn!("tried to read preferences from non-main thread");
        return PreferencesStream {
            _guards: Vec::default(),
            _accent_color: AccentColorObserver::default(),
            inner: stream::once(AvailablePreferences::default()).boxed(),
        };
    };

    #[allow(unused_mut)]
    let mut guards = Vec::new();
    let mut preferences = AvailablePreferences::default();
    let (sender, receiver) = mpsc::unbounded();

    #[cfg(feature = "reduced-motion")]
    if interest.is(Interest::ReducedMotion) {
        let sender = sender.clone();
        if let Some((guard, value)) = bool::boolean_media_query(
            &window,
            "(prefers-reduced-motion: reduce)",
            ReducedMotion::Reduce,
            ReducedMotion::NoPreference,
            move |v| _ = sender.unbounded_send(Preference::ReducedMotion(v)),
        ) {
            guards.push(guard);
            preferences.reduced_motion = value;
        }
    }

    #[cfg(feature = "reduced-transparency")]
    if interest.is(Interest::ReducedTransparency) {
        let sender = sender.clone();
        if let Some((guard, value)) = bool::boolean_media_query(
            &window,
            "(prefers-reduced-transparency: reduce)",
            ReducedTransparency::Reduce,
            ReducedTransparency::NoPreference,
            move |v| _ = sender.unbounded_send(Preference::ReducedTransparency(v)),
        ) {
            guards.push(guard);
            preferences.reduced_transparency = value;
        }
    }

    #[cfg(feature = "color-scheme")]
    if interest.is(Interest::ColorScheme) {
        let sender = sender.clone();
        if let Some((guards_, value)) = color_scheme_media_query(&window, move |v| {
            _ = sender.unbounded_send(Preference::ColorScheme(v))
        }) {
            guards.extend(guards_);
            preferences.color_scheme = value;
        }
    }

    #[cfg(feature = "contrast")]
    if interest.is(Interest::Contrast) {
        let sender = sender.clone();
        if let Some((guards_, value)) = contrast_media_query(&window, move |v| {
            _ = sender.unbounded_send(Preference::Contrast(v))
        }) {
            guards.extend(guards_);
            preferences.contrast = value;
        }
    }

    #[cfg(feature = "accent-color")]
    let accent_color = if interest.is(Interest::AccentColor) {
        let sender = sender.clone();
        let callback = move |v| _ = sender.unbounded_send(Preference::AccentColor(v));
        if let Some((observer, value)) = accent_color::AccentColorObserver::new(&window, callback) {
            preferences.accent_color = value;
            Some(DropOnMainThread::new(observer, &window))
        } else {
            None
        }
    } else {
        None
    };

    PreferencesStream {
        _guards: guards,
        #[cfg(feature = "accent-color")]
        _accent_color: accent_color,
        #[cfg(not(feature = "accent-color"))]
        _accent_color: (),
        inner: stream::once(preferences)
            .chain(changes(preferences, receiver))
            .boxed(),
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

#[derive(Debug, Clone, Copy)]
enum Preference {
    #[cfg(feature = "color-scheme")]
    ColorScheme(ColorScheme),
    #[cfg(feature = "contrast")]
    Contrast(Contrast),
    #[cfg(feature = "reduced-motion")]
    ReducedMotion(ReducedMotion),
    #[cfg(feature = "reduced-transparency")]
    ReducedTransparency(ReducedTransparency),
    #[cfg(feature = "accent-color")]
    AccentColor(AccentColor),
}

impl Preference {
    fn apply(self, mut preferences: AvailablePreferences) -> AvailablePreferences {
        match self {
            #[cfg(feature = "color-scheme")]
            Preference::ColorScheme(v) => preferences.color_scheme = v,
            #[cfg(feature = "contrast")]
            Preference::Contrast(v) => preferences.contrast = v,
            #[cfg(feature = "reduced-motion")]
            Preference::ReducedMotion(v) => preferences.reduced_motion = v,
            #[cfg(feature = "reduced-transparency")]
            Preference::ReducedTransparency(v) => preferences.reduced_transparency = v,
            #[cfg(feature = "accent-color")]
            Preference::AccentColor(v) => preferences.accent_color = v,
        };
        preferences
    }
}

#[cfg(feature = "contrast")]
multi_value_media_query! {
    contrast_media_query -> Contrast {
        "(prefers-contrast: more)" => Contrast::More,
        "(prefers-contrast: less)" => Contrast::Less,
        "(prefers-contrast: custom)" => Contrast::Custom,
        _ => Contrast::NoPreference,
    }
}

#[cfg(feature = "color-scheme")]
multi_value_media_query! {
    color_scheme_media_query -> ColorScheme {
        "(prefers-color-scheme: dark)" => ColorScheme::Dark,
        "(prefers-color-scheme: light)" => ColorScheme::Light,
        _ => ColorScheme::NoPreference,
    }
}
