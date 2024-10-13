use wasm_bindgen::prelude::Closure;
use wasm_bindgen::{JsCast, JsValue};
use web_sys::{Event, EventTarget};

pub(crate) trait EventTargetExt {
    fn add_event_listener<E: JsCast>(
        &self,
        type_: &'static str,
        f: impl FnMut(E) + 'static,
    ) -> Result<EventListenerGuard, JsValue>;
}

impl EventTargetExt for EventTarget {
    fn add_event_listener<E: JsCast>(
        &self,
        type_: &'static str,
        mut f: impl FnMut(E) + 'static,
    ) -> Result<EventListenerGuard, JsValue> {
        let listener =
            Closure::wrap(Box::new(move |event: Event| f(event.unchecked_into::<E>()))
                as Box<dyn FnMut(Event)>);
        self.add_event_listener_with_callback(type_, listener.as_ref().unchecked_ref())?;
        Ok(EventListenerGuard {
            target: self.clone(),
            type_,
            listener,
        })
    }
}

/// A registered event listener that is automatically
/// removed on drop.
pub(crate) struct EventListenerGuard {
    target: EventTarget,
    type_: &'static str,
    listener: Closure<dyn FnMut(Event)>,
}

impl Drop for EventListenerGuard {
    fn drop(&mut self) {
        _ = self
            .target
            .remove_event_listener_with_callback(self.type_, self.listener.as_ref().unchecked_ref())
    }
}
