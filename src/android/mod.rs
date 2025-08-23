use crate::{AvailablePreferences, Interest};
use futures_lite::stream;
use std::time::Duration;

// signatures: <https://docs.oracle.com/javase/8/docs/technotes/guides/jni/spec/types.html>

mod result;
mod subscription;
mod support;

pub(crate) type PreferencesStream = stream::Once<AvailablePreferences>;

pub(crate) fn stream(_interest: Interest) -> PreferencesStream {
    if let Err(_e) = subscription::subscribe(|| {
        #[cfg(feature = "log")]
        log::info!("preferences changed");
    }) {
        #[cfg(feature = "log")]
        log::warn!("failed to subscribe: {_e:#?}");
    }
    default_stream()
}

pub(crate) fn default_stream() -> PreferencesStream {
    stream::once(AvailablePreferences::default())
}

pub(crate) fn once_blocking(
    _interest: Interest,
    _timeout: Duration,
) -> Option<AvailablePreferences> {
    Some(AvailablePreferences::default())
}
