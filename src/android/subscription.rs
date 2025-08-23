use super::result::Result;
use super::support::{java_vm, JavaSupport};
use crate::callback_utils::{CallbackHandle, Callbacks};
use std::mem;
use std::panic::catch_unwind;
use std::sync::RwLock;

pub(crate) type CallbackFn = Box<dyn Fn() + Send + Sync>;

#[derive(Debug)]
pub(crate) struct Subscription {
    handle: CallbackHandle,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Err(_e) = unsubscribe(mem::take(&mut self.handle)) {
            #[cfg(feature = "log")]
            log::warn!("Error while unsubscribing: {_e:#?}");
        }
    }
}

static CALLBACKS: RwLock<Callbacks<CallbackFn>> = RwLock::new(Callbacks::new());

pub(crate) fn subscribe(callback: impl Fn() + Send + Sync + 'static) -> Result<Subscription> {
    let mut callbacks = CALLBACKS.write().expect("lock poisoned");
    if callbacks.is_empty() {
        subscribe_java()?;
    }
    let handle = callbacks.add(Box::new(callback))?;
    Ok(Subscription { handle })
}

fn subscribe_java() -> Result<()> {
    let vm = java_vm()?;
    let mut env = vm.attach_current_thread()?;
    let support = JavaSupport::get()?;
    support.subscribe(&mut env)?;
    Ok(())
}

fn unsubscribe(handle: CallbackHandle) -> Result<()> {
    let mut callbacks = CALLBACKS.write().expect("lock poisoned");
    callbacks.remove(handle);
    if callbacks.is_empty() {
        unsubscribe_java()?;
    }
    Ok(())
}

fn unsubscribe_java() -> Result<()> {
    let vm = java_vm()?;
    let mut env = vm.attach_current_thread()?;
    let support = JavaSupport::get()?;
    support.unsubscribe(&mut env)?;
    Ok(())
}

pub(crate) fn on_configuration_changed() {
    let Ok(callbacks) = CALLBACKS.read() else {
        return;
    };
    for callback in callbacks.iter() {
        callback();
    }
}

// This method is called from Java using `native` method.
//
// The [JNI Design Overview](https://docs.oracle.com/javase/1.5.0/docs/guide/jni/spec/design.html)
// documents the name mangling scheme.
#[no_mangle]
pub extern "C" fn Java_garden_tau_mundy_MundySupport_onPreferencesChanged() {
    _ = catch_unwind(on_configuration_changed);
}
