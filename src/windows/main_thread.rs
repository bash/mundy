use windows::Win32::System::Threading::GetCurrentThreadId;

// This code is copied from winit, licensed under the Apache 2.0 license.
// <https://github.com/rust-windowing/winit/blob/4e3165f3d81b1ee2771d517103c5883ffa2ee29f/src/platform_impl/windows/event_loop.rs>

/// Returns the id of the main thread.
///
/// Windows has no real API to check if the current executing thread is the "main thread", unlike
/// macOS.
///
/// Windows will let us look up the current thread's id, but there's no API that lets us check what
/// the id of the main thread is. We would somehow need to get the main thread's id before a
/// developer could spin off any other threads inside of the main entrypoint in order to emulate the
/// capabilities of other platforms.
///
/// We can get the id of the main thread by using CRT initialization. CRT initialization can be used
/// to setup global state within a program. The OS will call a list of function pointers which
/// assign values to a static variable. To have get a hold of the main thread id, we need to place
/// our function pointer inside of the `.CRT$XCU` section so it is called before the main
/// entrypoint.
///
/// Full details of CRT initialization can be found here:
/// <https://docs.microsoft.com/en-us/cpp/c-runtime-library/crt-initialization?view=msvc-160>
pub(crate) fn main_thread_id() -> u32 {
    static mut MAIN_THREAD_ID: u32 = 0;

    /// Function pointer used in CRT initialization section to set the above static field's value.
    // Mark as used so this is not removable.
    #[used]
    #[allow(non_upper_case_globals)]
    // Place the function pointer inside of CRT initialization section so it is loaded before
    // main entrypoint.
    //
    // See: https://doc.rust-lang.org/stable/reference/abi.html#the-link_section-attribute
    #[link_section = ".CRT$XCU"]
    static INIT_MAIN_THREAD_ID: unsafe fn() = {
        unsafe fn initer() {
            unsafe { MAIN_THREAD_ID = GetCurrentThreadId() };
        }
        initer
    };

    unsafe { MAIN_THREAD_ID }
}
