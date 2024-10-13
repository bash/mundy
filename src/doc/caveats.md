## Platform-specific Caveats
* macOS: You must call this function from the main thread and
  be careful to initialize winit's event loop before calling this function,
  see [#3772](https://github.com/rust-windowing/winit/issues/3772).
* Windows: You must have an event loop running on the main thread to receive
  updates when preferences are changed.
* Web: You must call this function from the main thread.
