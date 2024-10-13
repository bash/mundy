use crate::{async_rt, Interest, Preferences};
use futures_util::{stream, StreamExt as _};

pub trait CallbackFn: FnMut(Preferences) + Send + Sync + 'static {}

impl<F> CallbackFn for F where F: FnMut(Preferences) + Send + Sync + 'static {}

/// A subscription for preferences created using [`Preferences::subscribe()`].
/// Dropping the subscription will cancel it and clean up all associated resources.
pub struct Subscription(Option<stream::AbortHandle>);

#[cfg(test)]
static_assertions::assert_impl_all!(Subscription: Send, Sync);

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(handle) = &self.0 {
            handle.abort();
        }
    }
}

impl Preferences {
    /// Creates a new subscription for a selection of system preferences given by `interests`.
    ///
    /// The provided callback is guaranteed to be called at least once with the initial values
    /// and is subsequently called when preferences are updated.
    ///
    #[doc = include_str!("doc/caveats.md")]
    pub fn subscribe(interest: Interest, mut callback: impl CallbackFn) -> Subscription {
        // No need to spawn a thread if the interests are empty.
        if interest.is_empty() {
            return Subscription(None);
        }
        let (mut stream, handle) = stream::abortable(Self::stream(interest));
        async_rt::spawn_future(async move {
            while let Some(p) = stream.next().await {
                callback(p);
            }
        });
        Subscription(Some(handle))
    }
}
