//! # Android
//! ## Overview
//! The Android backend of mundy uses the Java Native Interface (JNI) to call the Java-based APIs.
//! Some limited things are exposed as C APIs through the [NDK] with its corresponding [`ndk`] crate.
//!
//! To interact with Java code we need two things, both of which are provided to us by the
//! [`ndk_context`] crate:
//! * the JVM instance
//! * the current Android [`Context`].
//!
//! The [`ndk_context`] crate works with both apps written in Rust and apps
//! written in Java/Kotlin that call into Rust.
//!
//! The facilitate interacting with the Java APIs, we have some glue code
//! written in Java, that gets compiled to DEX bytecode in our `build.rs`
//! and then injected at runtime in the [`support`] module.
//!
//! A lot of this is based on the setup that the [`netwatcher`] crate uses.
//! There's also an excellent [blog post] by crate's author, Thomas Karpiniec.
//!
//! ## Caveats
//!
//! ### Activity re-creation
//! Certain actions on Android, such as changing the system theme (accent color, etc.)
//! results in the activity being destroyed and re-created.
//!
//! TODO: do we need to be informed about activity re-creation for java/kotlin apps?
//!
//! For applications written using [`NativeActivity`] this means that
//! `android_main` is expected to return after receiving a [`Destroy`] event. It is then called again with
//! the new activity. Winit currently [does not handle the `Destroy` event appropriately][winit-bug], causing apps to freeze.
//!
//! ### Configuration Changes
//!
//! For some [configuration changes](https://developer.android.com/guide/topics/resources/runtime-changes)
//! there is no way to subscribe to, other than having access to the activity (e.g. subclassing or setting a method in the vtable in the case of [`NativeActivity`]).
//! This is the case for settings like [`uiMode`] (Dark mode versus light mode).
//!
//! For settings like these, we rely on the user of mundy to call [`crate::platform::android::on_configuration_changed`].
//!
//! Unfortunately, `winit` [does not provide][winit-missing-api] access to the `ConfigurationChanged` event.
//! So apps relying on `winit` will not be able to detect dark/light mode changes.
//!
//! [NDK]: https://developer.android.com/ndk/reference
//! [`ndk`]: https://docs.rs/ndk/0.9.0/ndk/
//! [`Context`]: https://developer.android.com/reference/android/content/Context
//! [blog post]: https://octet-stream.net/b/scb/2025-08-03-injecting-java-from-native-libraries-on-android.html
//! [`netwatcher`]: https://github.com/thombles/netwatcher
//! [winit-bug]: https://github.com/rust-windowing/winit/issues/4303
//! [winit-missing-api]: https://github.com/rust-windowing/winit/issues/2120
//! [`Destroy`]: https://docs.rs/android-activity/latest/android_activity/enum.MainEvent.html#variant.Destroy
//! [`NativeActivity`]: https://developer.android.com/reference/android/app/NativeActivity
//! [`uiMode`]: https://developer.android.com/reference/android/content/res/Configuration#uiMode

#[cfg(feature = "color-scheme")]
use crate::ColorScheme;
#[cfg(feature = "contrast")]
use crate::Contrast;
#[cfg(feature = "reduced-motion")]
use crate::ReducedMotion;
#[cfg(feature = "accent-color")]
use crate::{AccentColor, Srgba};
use crate::{AvailablePreferences, Interest};
use futures_channel::mpsc;
use futures_lite::{stream, Stream, StreamExt as _};
use jni::JNIEnv;
use pin_project_lite::pin_project;
use result::Result;
use std::time::Duration;
use support::{java_vm, JavaSupport};

// signatures: <https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/types.html>

mod result;
mod subscription;
mod support;
pub(crate) use subscription::on_configuration_changed;

pin_project! {
    pub(crate) struct PreferencesStream {
        subscription: Option<subscription::Subscription>,
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
    let (tx, rx) = mpsc::unbounded();
    let subscription = match subscription::subscribe(move || send_preferences(interest, &tx)) {
        Ok(subscription) => subscription,
        #[cfg(not(feature = "log"))]
        Err(_) => return default_stream(),
        #[cfg(feature = "log")]
        Err(e) => {
            #[cfg(feature = "log")]
            log::warn!("failed to subscribe for preference changes: {e:#?}");
            return default_stream();
        }
    };

    PreferencesStream {
        subscription: Some(subscription),
        inner: stream::once(get_preferences(interest)).chain(rx).boxed(),
    }
}

pub(crate) fn default_stream() -> PreferencesStream {
    PreferencesStream {
        subscription: None,
        inner: stream::once(AvailablePreferences::default()).boxed(),
    }
}

pub(crate) fn once_blocking(
    interest: Interest,
    _timeout: Duration,
) -> Option<AvailablePreferences> {
    Some(get_preferences(interest))
}

fn send_preferences(interest: Interest, tx: &mpsc::UnboundedSender<AvailablePreferences>) {
    _ = tx.unbounded_send(get_preferences(interest));
}

fn get_preferences(interest: Interest) -> AvailablePreferences {
    let result = try_get_preferences(interest);
    #[cfg(feature = "log")]
    if let Err(e) = &result {
        log::warn!("failed to get preferences: {e:#?}");
    }
    result.unwrap_or_default()
}

fn try_get_preferences(interest: Interest) -> Result<AvailablePreferences> {
    let vm = java_vm()?;
    let mut env = vm.attach_current_thread()?;
    let support = JavaSupport::get()?;

    let mut preferences = AvailablePreferences::default();

    #[cfg(feature = "color-scheme")]
    if interest.is(Interest::ColorScheme) {
        preferences.color_scheme = get_color_scheme(&support, &mut env).unwrap_or_default();
    }

    #[cfg(feature = "contrast")]
    if interest.is(Interest::Contrast) {
        preferences.contrast = get_contrast(&support, &mut env).unwrap_or_default();
    }

    #[cfg(feature = "reduced-motion")]
    if interest.is(Interest::ReducedMotion) {
        preferences.reduced_motion = get_reduced_motion(&support, &mut env).unwrap_or_default();
    }

    #[cfg(feature = "reduced-motion")]
    if interest.is(Interest::AccentColor) {
        preferences.accent_color = get_accent_color(&support, &mut env).unwrap_or_default();
    }

    Ok(preferences)
}

#[cfg(feature = "color-scheme")]
fn get_color_scheme(support: &JavaSupport, env: &mut JNIEnv) -> Result<ColorScheme> {
    if support.get_night_mode(env)? {
        Ok(ColorScheme::Dark)
    } else {
        Ok(ColorScheme::Light)
    }
}

#[cfg(feature = "contrast")]
fn get_contrast(support: &JavaSupport, env: &mut JNIEnv) -> Result<Contrast> {
    if support.get_high_contrast(env)? {
        Ok(Contrast::More)
    } else {
        Ok(Contrast::NoPreference)
    }
}

#[cfg(feature = "reduced-motion")]
fn get_reduced_motion(support: &JavaSupport, env: &mut JNIEnv) -> Result<ReducedMotion> {
    if support.get_prefers_reduced_motion(env)? {
        Ok(ReducedMotion::Reduce)
    } else {
        Ok(ReducedMotion::NoPreference)
    }
}

#[cfg(feature = "reduced-motion")]
fn get_accent_color(support: &JavaSupport, env: &mut JNIEnv) -> Result<AccentColor> {
    let color = support.get_accent_color(env)? as u32;
    // Color ints in Android APIs always define colors in the
    // sRGB color space, packed into an int as #AARRGGBB:
    // https://developer.android.com/reference/android/graphics/Color#color-ints
    let alpha = ((color >> 24) & 0xff) as u8;
    let red = ((color >> 16) & 0xff) as u8;
    let green = ((color >> 8) & 0xff) as u8;
    let blue = (color & 0xff) as u8;
    let color = Srgba::from_u8_array([red, green, blue, alpha]);
    Ok(AccentColor(Some(color)))
}
