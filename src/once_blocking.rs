use crate::imp;
use crate::{Interest, Preferences};
use std::time::Duration;

impl Preferences {
    /// TODO
    ///
    #[doc = include_str!("doc/caveats.md")]
    pub fn once_blocking(interest: Interest, timeout: Duration) -> Option<Self> {
        if interest.is_empty() {
            return Some(Default::default());
        }
        imp::once_blocking(interest, timeout).map(Self::from)
    }
}
