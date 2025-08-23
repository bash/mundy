#[cfg(feature = "color-scheme")]
use crate::ColorScheme;
#[cfg(feature = "contrast")]
use crate::Contrast;
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
    log::info!("sending preferences w. interest {interest:#?} to tx: {tx:#?}");
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
