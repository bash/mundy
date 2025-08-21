use crate::{AvailablePreferences, Interest};
use futures_lite::stream;
use std::time::Duration;

pub(crate) type PreferencesStream = stream::Once<AvailablePreferences>;

pub(crate) fn stream(_interest: Interest) -> PreferencesStream {
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
