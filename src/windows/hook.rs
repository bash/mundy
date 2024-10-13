//! Sets up a [Windows Hook](https://learn.microsoft.com/en-us/windows/win32/winmsg/about-hooks)
//! to intercept messages (we care about `WM_SETTINGCHANGE` in particular).
//! This is a lot easier (and involves a lot less unsafe code) than setting
//! up our own hidden window and event loop.

use crate::windows::main_thread::main_thread_id;
use slab::Slab;
use std::panic::catch_unwind;
use std::sync::{Arc, RwLock, Weak};
use windows::core::Owned;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, CWPSTRUCT, HHOOK, WH_CALLWNDPROC,
};

/// Registers a windows hook that is automatically unregistered when
/// the returned guard is dropped.
pub(crate) fn register_windows_hook(
    hook: CallbackFn,
) -> Result<WindowsHookGuard, windows::core::Error> {
    let callback = register_callback(hook);
    let hook = register_hook()?;
    Ok(WindowsHookGuard((hook, callback)))
}

pub(crate) type CallbackFn = Box<dyn Fn(CWPSTRUCT) + Send + Sync>;

pub(crate) struct WindowsHookGuard(
    #[expect(dead_code, reason = "used to free resources on drop")]
    (Arc<HookHandle>, CallbackHandle),
);

static CALLBACKS: RwLock<Slab<CallbackFn>> = RwLock::new(Slab::new());

unsafe extern "system" fn hook_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    // SAFETY: lParam: A pointer to a CWPSTRUCT structure that contains details about the message.
    let data = unsafe { *(lparam.0 as *const CWPSTRUCT) };
    _ = catch_unwind(|| {
        if let Ok(callbacks) = CALLBACKS.read() {
            for (_, callback) in callbacks.iter() {
                callback(data);
            }
        }
    });
    unsafe { CallNextHookEx(None, ncode, wparam, lparam) }
}

fn register_hook() -> Result<Arc<HookHandle>, windows::core::Error> {
    static WEAK: RwLock<Weak<HookHandle>> = RwLock::new(Weak::new());
    let mut weak = WEAK.write().expect("lock poisoned");

    if let Some(hook) = weak.upgrade() {
        Ok(hook)
    } else {
        // SAFETY:
        // * hook_proc is a valid fn pointer
        // * we're the owners of the returned handle
        let handle = unsafe {
            Owned::new(SetWindowsHookExW(
                WH_CALLWNDPROC,
                Some(hook_proc),
                None,
                main_thread_id(),
            )?)
        };
        let hook = Arc::new(HookHandle(handle));
        *weak = Arc::downgrade(&hook);
        Ok(hook)
    }
}

struct HookHandle(#[expect(dead_code, reason = "used to free resources on drop")] Owned<HHOOK>);

// SAFETY: Only used to free the hook on drop.
unsafe impl Send for HookHandle {}

// SAFETY: Only used to free the hook on drop.
unsafe impl Sync for HookHandle {}

fn register_callback(callback: CallbackFn) -> CallbackHandle {
    let mut callbacks = CALLBACKS.write().expect("lock poisoned");
    CallbackHandle(callbacks.insert(callback))
}

struct CallbackHandle(usize);

impl Drop for CallbackHandle {
    fn drop(&mut self) {
        if let Ok(mut callbacks) = CALLBACKS.write() {
            _ = callbacks.remove(self.0);
        }
    }
}
