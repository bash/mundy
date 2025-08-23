# Running the Examples

## winit

Running the [winit](https://github.com/rust-windowing/winit) example is as easy as:
```shell
cargo run -p winit-example
```

## egui

Running the [egui](https://www.egui.rs/) example is as easy as:
```shell
cargo run -p egui-example
```

## Web

You can run the web examples using [Trunk](https://trunkrs.dev/):
```shell
trunk serve examples/wasm/index.html
# or
trunk serve examples/egui/index.html
```

## Android

The easiest way to run the examples on Android
is to use [xbuild](https://github.com/rust-mobile/xbuild):

```shell
x run --device adb:emulator-... -p winit-example
x run --device adb:emulator-... -p egui-example
```
