use crate::{AvailablePreferences, Interest};
use futures_lite::stream;

pub(crate) type PreferencesStream = stream::Once<AvailablePreferences>;

pub(crate) fn stream(_interest: Interest) -> PreferencesStream {
    stream::once(AvailablePreferences::default())
}
