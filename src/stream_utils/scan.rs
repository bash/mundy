//! This file was copied/adapted from the futures-rs project
//! and is licensed under either of Apache License, Version 2.0 or MIT license.
//! <https://github.com/rust-lang/futures-rs>

#![allow(clippy::unwrap_used)]

#[path = "unfold_state.rs"]
mod unfold_state;

use core::fmt;
use core::pin::Pin;
use futures_lite::future::Future;
use futures_lite::{ready, stream, Stream};
use pin_project_lite::pin_project;
use std::task::{Context, Poll};
use unfold_state::UnfoldState;

pin_project! {
    #[must_use = "streams do nothing unless polled"]
    pub struct Scan<St: Stream, S, Fut, F> {
        #[pin]
        stream: St,
        f: F,
        #[pin]
        state: UnfoldState<S, Fut>,
    }
}

impl<St, S, Fut, F> fmt::Debug for Scan<St, S, Fut, F>
where
    St: Stream + fmt::Debug,
    St::Item: fmt::Debug,
    S: fmt::Debug,
    Fut: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Scan")
            .field("stream", &self.stream)
            .field("state", &self.state)
            .field("done_taking", &self.is_done_taking())
            .finish()
    }
}

impl<St: Stream, S, Fut, F> Scan<St, S, Fut, F> {
    /// Checks if internal state is `None`.
    fn is_done_taking(&self) -> bool {
        self.state.is_empty()
    }
}

impl<B, St, S, Fut, F> Scan<St, S, Fut, F>
where
    St: Stream,
    F: FnMut(S, St::Item) -> Fut,
    Fut: Future<Output = Option<(S, B)>>,
{
    pub(crate) fn new(stream: St, initial_state: S, f: F) -> Self {
        Self {
            stream,
            f,
            state: UnfoldState::Value {
                value: initial_state,
            },
        }
    }
}

impl<B, St, S, Fut, F> Stream for Scan<St, S, Fut, F>
where
    St: Stream,
    F: FnMut(S, St::Item) -> Fut,
    Fut: Future<Output = Option<(S, B)>>,
{
    type Item = B;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<B>> {
        if self.is_done_taking() {
            return Poll::Ready(None);
        }

        let mut this = self.project();

        Poll::Ready(loop {
            if let Some(fut) = this.state.as_mut().project_future() {
                match ready!(fut.poll(cx)) {
                    None => {
                        this.state.set(UnfoldState::Empty);
                        break None;
                    }
                    Some((state, item)) => {
                        this.state.set(UnfoldState::Value { value: state });
                        break Some(item);
                    }
                }
            } else if let Some(item) = ready!(this.stream.as_mut().poll_next(cx)) {
                let state = this.state.as_mut().take_value().unwrap();
                this.state.set(UnfoldState::Future {
                    future: (this.f)(state, item),
                })
            } else {
                break None;
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        if self.is_done_taking() {
            (0, Some(0))
        } else {
            self.stream.size_hint() // can't know a lower bound, due to the predicate
        }
    }
}
