use futures_lite::Stream;
use pin_project_lite::pin_project;

#[allow(non_snake_case)]
pub fn Left<L, R>(inner: L) -> Either<L, R> {
    Either::Left { inner }
}

#[allow(non_snake_case)]
pub fn Right<L, R>(inner: R) -> Either<L, R> {
    Either::Right { inner }
}

pin_project! {
    #[project = EitherProj]
    pub(crate) enum Either<L, R> {
        Left { #[pin] inner: L },
        Right { #[pin] inner: R },
    }
}

impl<T, L, R> Stream for Either<L, R>
where
    L: Stream<Item = T>,
    R: Stream<Item = T>,
{
    type Item = T;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.project() {
            EitherProj::Left { inner } => inner.poll_next(cx),
            EitherProj::Right { inner } => inner.poll_next(cx),
        }
    }
}
