fn main() {
    if cargo::target_os() == "android" {
        android::build();
    }
}

// A lot of this is thankfully adopted from netwatcher's [build.rs](https://github.com/thombles/netwatcher/blob/f1353ba6b9a9e4e28a223a317564a3b34a649aae/Cargo.toml).
mod android {
    use crate::cargo;
    use android_build::{DebugInfo, Dexer, JavaBuild};

    pub(super) fn build() {
        let android_jar =
            android_build::android_jar(None).expect("Unable to locate android.jar path");
        let java_source_files = &[
            "src/android/MundySupport.java",
            "src/android/MundyBackgroundThread.java",
        ];
        for f in java_source_files {
            println!("cargo:rerun-if-changed={f}");
        }
        let out_dir = cargo::out_dir();
        let classes_out_dir = out_dir.join("java/garden/tau/mundy");
        let _ = std::fs::remove_dir_all(&classes_out_dir);
        std::fs::create_dir_all(&classes_out_dir).unwrap();
        JavaBuild::new()
            .files(java_source_files)
            .class_path(&android_jar)
            .classes_out_dir(&classes_out_dir)
            .java_source_version(8)
            .java_target_version(8)
            .debug_info(debug_info())
            .compile()
            .expect("java build failed");

        let dex_output_dir = out_dir.join("dex");
        let _ = std::fs::remove_dir_all(&dex_output_dir);
        std::fs::create_dir_all(&dex_output_dir).unwrap();
        let java_classes_root = out_dir.join("java");

        Dexer::new()
            .android_jar(&android_jar)
            .class_path(&java_classes_root)
            .collect_classes(&java_classes_root)
            .unwrap()
            .release(cargo::is_release_profile())
            .android_min_api(21)
            .out_dir(&dex_output_dir)
            .run()
            .expect("dexing failed");

        let dex_path = dex_output_dir.join("classes.dex");
        if dex_path.exists() {
            println!("cargo:rustc-env=MUNDY_DEX_PATH={}", dex_path.display());
        } else {
            panic!(
                "DEX file was not created at expected location: {}",
                dex_path.display()
            );
        }
    }

    fn debug_info() -> DebugInfo {
        let is_release_build = cargo::is_release_profile();
        DebugInfo {
            line_numbers: !is_release_build,
            source_files: !is_release_build,
            variables: !is_release_build,
        }
    }
}

mod cargo {
    use std::env;
    use std::path::PathBuf;

    pub(super) fn is_release_profile() -> bool {
        env::var("PROFILE").is_ok_and(|p| p == "release")
    }

    pub(super) fn out_dir() -> PathBuf {
        env::var_os("OUT_DIR").unwrap().into()
    }

    pub(super) fn target_os() -> String {
        env::var("CARGO_CFG_TARGET_OS").unwrap()
    }
}
