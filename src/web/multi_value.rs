use super::drop_on_main_thread::DropOnMainThread;
use super::event_listener::{EventListenerGuard, EventTargetExt as _};
use web_sys::{MediaQueryListEvent, Window};

/// Generates a media query function for a media query
/// with multiple values (e.g. `prefers-contrast`).
/// It's future proof to addition of new keywords (i.e. it treats such new values as no preference).
macro_rules! multi_value_media_query {
    ($name:ident -> $ty:ty { $($query:literal => $value:expr,)* _ => $default:expr $(,)? }) => {
        fn $name(
            window: &web_sys::Window,
            callback: impl FnMut($ty) + Clone + 'static,
        ) -> Option<(Vec<DropOnMainThread<EventListenerGuard>>, $ty)> {
            use $crate::web::multi_value::single_value_media_query;

            let mut guards = Vec::new();
            let mut initial_value = $default;

            $(
                let (guard, v) = single_value_media_query(window, $query, $value, callback.clone())?;
                if let Some(v) = v { initial_value = v; }
                guards.push(guard);
            )*

            // We don't use "no-preference" here. Instead we use a logical combination of all the values
            // we know. That way when the standard adds a new value, we treat it as no-preference.
            const NO_PREFERENCE: &str = concat!("all", $(" and (not ", $query, ")"),*);
            guards.push(single_value_media_query(window, NO_PREFERENCE, $default, callback)?.0);

            Some((guards, initial_value))
        }
    };
}

#[doc(hidden)]
pub(crate) fn single_value_media_query<T: Copy + 'static>(
    window: &Window,
    query: &str,
    value: T,
    mut callback: impl FnMut(T) + Clone + 'static,
) -> Option<(DropOnMainThread<EventListenerGuard>, Option<T>)> {
    let media_query = window.match_media(query).ok().flatten()?;
    let listener = move |event: MediaQueryListEvent| {
        if event.matches() {
            callback(value);
        }
    };
    let guard = media_query.add_event_listener("change", listener).ok()?;
    let initial_value = media_query.matches().then_some(value);
    Some((DropOnMainThread::new(guard, window), initial_value))
}
