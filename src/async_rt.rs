use cfg_if::cfg_if;
use std::future::Future;

cfg_if! {
    if #[cfg(all(feature = "tokio", target_os = "linux"))] {
        pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
            // These are the features that zbus also activates:
            // <https://github.com/dbus2/zbus/blob/4e4151a9f9d745803d0337e1cd73e2b0f8eaec0d/zbus/src/utils.rs#L39>
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
                .expect("launch of single-threaded tokio runtime");
            runtime.block_on(future)
        }
    } else if #[cfg(all(feature = "async-io", target_os = "linux"))] {
        pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
            async_io::block_on(future)
        }
    } else if #[cfg(not(all(target_family = "wasm", target_os = "unknown")))] {
        pub(crate) fn block_on<F: Future>(future: F) -> F::Output {
            beul::execute(future)
        }
    }
}

#[cfg(all(target_family = "wasm", target_os = "unknown"))]
pub(crate) fn spawn_future(future: impl Future<Output = ()> + Send + 'static) {
    wasm_bindgen_futures::spawn_local(future);
}

#[cfg(not(all(target_family = "wasm", target_os = "unknown")))]
pub(crate) fn spawn_future(future: impl Future<Output = ()> + Send + 'static) {
    std::thread::Builder::new()
        .name(format!("{} subscription thread", env!("CARGO_PKG_NAME")))
        .spawn(move || block_on(future))
        .expect("failed to spawn thread");
}
