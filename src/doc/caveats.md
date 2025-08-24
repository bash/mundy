<details>

<summary style="cursor: pointer">

## Platform-specific Caveats

</summary>

* macOS: You must call this function from the main thread.
* Windows: You must have an event loop running on the main thread to receive
  updates when preferences are changed.
* Web: You must call this function from the main thread.
* Android: You must initialize [`ndk-context`] first. See the [`platform::android`] module for details.

</details>

[`ndk-context`]: https://docs.rs/ndk-context
[`platform::android`]: crate::platform::android
