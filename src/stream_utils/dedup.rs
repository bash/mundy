use futures_lite::ready;
use futures_lite::stream::Stream;
use pin_project_lite::pin_project;
use std::pin::Pin;
use std::task::{Context, Poll};

pin_project! {
    pub struct Dedup<S: Stream> {
        #[pin]
        stream: S,
        current: Option<<S as Stream>::Item>,
    }
}

impl<S: Stream> Dedup<S> {
    pub(crate) fn new(stream: S) -> Self {
        Dedup {
            stream,
            current: None,
        }
    }
}

impl<S> Stream for Dedup<S>
where
    S: Stream,
    S::Item: Clone,
    S::Item: PartialEq<S::Item>,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let next = ready!(this.stream.poll_next(cx));

        match next {
            Some(v) if this.current.as_ref() != Some(&v) => {
                *this.current = Some(v.clone());
                Poll::Ready(Some(v))
            }
            Some(_) => {
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            None => Poll::Ready(None),
        }
    }
}
