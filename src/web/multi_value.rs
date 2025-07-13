use super::drop_on_main_thread::DropOnMainThread;
use super::event_listener::{EventListenerGuard, EventTargetExt as _};
use web_sys::{MediaQueryList, MediaQueryListEvent, Window};

/// Generates a media query function for a media query
/// with multiple values (e.g. `prefers-contrast`).
/// It's future proof to addition of new keywords (i.e. it treats such new values as no preference).
macro_rules! multi_value_media_query {
    ($name:ident -> $ty:ty { $($query:literal => $value:expr,)* _ => $default:expr $(,)? }) => {
        fn $name(window: &web_sys::Window) -> Option<$crate::web::multi_value::MultiValueMediaQuery<'_, $ty>> {
            use $crate::web::multi_value::{SingleValueMediaQuery, MultiValueMediaQuery};

            // We don't use "no-preference" here. Instead we use a logical combination of all the values
            // we know. That way when the standard adds a new value, we treat it as no-preference.
            const NO_PREFERENCE: &str = concat!("all", $(" and (not ", $query, ")"),*);

            let queries = vec![
                $(
                    SingleValueMediaQuery::new(window, $query, $value)?,
                )*
                SingleValueMediaQuery::new(window, NO_PREFERENCE, $default)?,
            ];

            Some(MultiValueMediaQuery::new($default, queries))
        }
    };
}

pub(crate) struct MultiValueMediaQuery<'a, T> {
    default: T,
    queries: Vec<SingleValueMediaQuery<'a, T>>,
}

impl<'a, T> MultiValueMediaQuery<'a, T>
where
    T: 'static + Copy,
{
    #[doc(hidden)]
    pub(crate) fn new(default: T, queries: Vec<SingleValueMediaQuery<'a, T>>) -> Self {
        Self { default, queries }
    }

    pub(crate) fn value(&self) -> T {
        self.queries
            .iter()
            .filter_map(|q| q.value())
            .next()
            .unwrap_or(self.default)
    }

    pub(crate) fn subscribe(
        self,
        callback: impl FnMut(T) + Clone + 'static,
    ) -> Option<Vec<DropOnMainThread<EventListenerGuard>>> {
        self.queries
            .into_iter()
            .map(|q| q.subscribe(callback.clone()))
            .collect()
    }
}

#[doc(hidden)]
pub(crate) struct SingleValueMediaQuery<'a, T> {
    window: &'a Window,
    query: MediaQueryList,
    value: T,
}

impl<'a, T> SingleValueMediaQuery<'a, T>
where
    T: Copy + 'static,
{
    pub(crate) fn new(window: &'a Window, query: &str, value: T) -> Option<Self> {
        Some(Self {
            window,
            value,
            query: window.match_media(query).ok().flatten()?,
        })
    }

    pub(crate) fn value(&self) -> Option<T> {
        self.query.matches().then_some(self.value)
    }

    pub(crate) fn subscribe(
        self,
        mut callback: impl FnMut(T) + Clone + 'static,
    ) -> Option<DropOnMainThread<EventListenerGuard>> {
        let listener = move |event: MediaQueryListEvent| {
            if event.matches() {
                callback(self.value);
            }
        };
        let guard = self.query.add_event_listener("change", listener).ok()?;
        Some(DropOnMainThread::new(guard, self.window))
    }
}
