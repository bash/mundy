use super::drop_on_main_thread::DropOnMainThread;
use super::event_listener::{EventListenerGuard, EventTargetExt as _};
use web_sys::{MediaQueryList, MediaQueryListEvent, Window};

pub(crate) struct BooleanMediaQuery<'a, T> {
    window: &'a Window,
    media_query_list: MediaQueryList,
    truthy: T,
    falsy: T,
}

impl<'a, T> BooleanMediaQuery<'a, T>
where
    T: 'static + Copy,
{
    pub(crate) fn new(
        window: &'a Window,
        query: &'static str,
        truthy: T,
        falsy: T,
    ) -> Option<Self> {
        let media_query_list = window.match_media(query).ok().flatten()?;
        Some(Self {
            window,
            media_query_list,
            truthy,
            falsy,
        })
    }

    pub(crate) fn value(&self) -> T {
        if self.media_query_list.matches() {
            self.truthy
        } else {
            self.falsy
        }
    }

    pub(crate) fn subscribe(
        self,
        mut callback: impl FnMut(T) + 'static,
    ) -> Option<DropOnMainThread<EventListenerGuard>> {
        let listener = {
            move |event: MediaQueryListEvent| {
                let value = if event.matches() {
                    self.truthy
                } else {
                    self.falsy
                };
                callback(value);
            }
        };
        let guard = self
            .media_query_list
            .add_event_listener("change", listener)
            .ok()?;
        Some(DropOnMainThread::new(guard, self.window))
    }
}
