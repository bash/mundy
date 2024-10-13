use super::drop_on_main_thread::DropOnMainThread;
use super::event_listener::{EventListenerGuard, EventTargetExt as _};
use web_sys::{MediaQueryListEvent, Window};

pub(crate) fn boolean_media_query<T: Copy + 'static>(
    window: &Window,
    query: &'static str,
    truthy: T,
    falsy: T,
    mut callback: impl FnMut(T) + 'static,
) -> Option<(DropOnMainThread<EventListenerGuard>, T)> {
    let media_query_list = window.match_media(query).ok().flatten()?;

    let listener = {
        move |event: MediaQueryListEvent| {
            let value = if event.matches() { truthy } else { falsy };
            callback(value);
        }
    };
    let guard = media_query_list
        .add_event_listener("change", listener)
        .ok()?;

    let initial_value = if media_query_list.matches() {
        truthy
    } else {
        falsy
    };
    Some((DropOnMainThread::new(guard, window), initial_value))
}
