use futures_channel::mpsc;
use futures_lite::StreamExt as _;
use std::any::Any;
use std::mem;
use std::sync::OnceLock;
use wasm_bindgen_futures::spawn_local;
use web_sys::{window, Window};

// The trick here is stolen straight from winit:
// https://github.com/rust-windowing/winit/blob/master/src/platform_impl/web/main_thread.rs

pub(crate) struct DropOnMainThread<T: Any>(Option<T>);

impl<T: Any> DropOnMainThread<T> {
    /// The `window` parameter is used to enforce this
    /// function to be called from the main thread.
    pub fn new(value: T, _window: &Window) -> Self {
        DROP_ZONE.get_or_init(|| {
            let (sender, mut receiver) = mpsc::unbounded();
            spawn_local(async move { while receiver.next().await.is_some() {} });
            sender
        });
        DropOnMainThread(Some(value))
    }
}

impl<T: Any> Drop for DropOnMainThread<T> {
    fn drop(&mut self) {
        if let Some(value) = self.0.take() {
            if mem::needs_drop::<T>() && window().is_none() {
                DROP_ZONE
                    .get()
                    .expect("drop zone not initialized")
                    .unbounded_send(DropBox(Box::new(value)))
                    .expect("failed to send value to main thread")
            }
        }
    }
}

// SAFETY: The inner value is send to the main thread in case
// Drop is called from another thread. The inner value is otherwise
// never accessible.
unsafe impl<T: Any> Send for DropOnMainThread<T> {}

static DROP_ZONE: OnceLock<mpsc::UnboundedSender<DropBox>> = OnceLock::new();

struct DropBox(#[expect(dead_code, reason = "field is only ever used for drop")] Box<dyn Any>);

// SAFETY: Only used to "send" a value back to the main thread where it was created.
// DropOnMainThread::new ensures that the value can only be created on the main thread.
unsafe impl Send for DropBox {}
