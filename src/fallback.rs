use crate::{AvailablePreferences, Interest};
use futures_util::{future, stream};

pub(crate) type PreferencesStream = stream::Once<future::Ready<AvailablePreferences>>;

pub(crate) fn stream(_interest: Interest) -> PreferencesStream {
    stream::once(future::ready(AvailablePreferences::default()))
}
