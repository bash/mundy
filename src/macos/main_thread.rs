use dispatch::Queue;
use objc2_foundation::MainThreadMarker;

// Inspired by run_on_main:
// <https://github.com/madsmtm/objc2/blob/67a4acd391663a072061f04d35ba2c1a351d8900/framework-crates/objc2-foundation/src/main_thread_bound.rs#L27>
pub(crate) fn run_on_main_async(f: impl FnOnce(MainThreadMarker) + Send + 'static) {
    Queue::main().exec_async(move || {
        // SAFETY: The outer closure is submitted to be run on the main thread.
        let mtm = unsafe { MainThreadMarker::new_unchecked() };
        f(mtm);
    });
}
