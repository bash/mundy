use crate::{async_rt, Interest, Preferences};
use futures_channel::oneshot;
use futures_lite::{stream, StreamExt as _};

pub trait CallbackFn: FnMut(Preferences) + Send + Sync + 'static {}

impl<F> CallbackFn for F where F: FnMut(Preferences) + Send + Sync + 'static {}

/// A subscription for preferences created using [`Preferences::subscribe()`].
/// Dropping the subscription will cancel it and clean up all associated resources.
pub struct Subscription(
    #[expect(dead_code, reason = "only used to send a canceled message on drop")]
    Option<oneshot::Sender<()>>,
);

#[cfg(test)]
static_assertions::assert_impl_all!(Subscription: Send, Sync);

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
        let (sender, receiver) = oneshot::channel();
        let mut stream = Self::stream(interest)
            .map(Message::Preferences)
            .race(stream::once_future(receiver).map(|_| Message::Shutdown));
        async_rt::spawn_future(async move {
            while let Some(message) = stream.next().await {
                match message {
                    Message::Preferences(preferences) => callback(preferences),
                    Message::Shutdown => break,
                }
            }
        });
        Subscription(Some(sender))
    }
}

enum Message {
    Preferences(Preferences),
    Shutdown,
}
