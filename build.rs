use std::env;
use std::path::PathBuf;

fn main() {
    // Determine the compilation target upfront.
    let target = env::var("TARGET").expect("TARGET environment variable not set");

    if target.contains("windows") {
        // Get the SDK path relative to the crate root.
        // Use platform-agnostic path joining
        let sdk_path = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .parent()
            .unwrap()
            .join("Connect_SDK")
            .join("Lib");

        let lib_dir = if target.contains("x86_64") {
            sdk_path.join("v120")
        } else {
            sdk_path.join("v120") // Adjust if 32-bit uses a different subfolder
        };

        // Tell cargo to look for shared libraries in the lib directory.
        println!("cargo:rustc-link-search=native={}", lib_dir.display());

        // Tell cargo to link the appropriate ImagerIPC2 library.
        if target.contains("x86_64") {
            println!("cargo:rustc-link-lib=dylib=ImagerIPC2x64");
        } else {
            println!("cargo:rustc-link-lib=dylib=ImagerIPC2");
        }

        // Bindings are now manually defined in src/ffi.rs.
        // This build script now only needs to link the dynamic library.

        // Run post-build script to copy DLL (debug builds only)
        if cfg!(debug_assertions) {
            let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
            let target_dir = manifest_dir.join("target").join("debug");
            let sdk_lib_dir = manifest_dir
                .parent()
                .unwrap()
                .join("Connect_SDK")
                .join("Lib")
                .join("v120");
            let dll_name = "ImagerIPC2x64.dll";
            let source_path = sdk_lib_dir.join(dll_name);
            let dest_path = target_dir.join(dll_name);

            if !target_dir.exists() {
                std::fs::create_dir_all(&target_dir)
                    .expect("Failed to create target/debug directory");
            }

            if source_path.exists() {
                std::fs::copy(&source_path, &dest_path).expect("Failed to copy DLL");
            } else {
                println!(
                    "cargo:warning=ImagerIPC2x64.dll not found at {}. Runtime camera support may be unavailable.",
                    source_path.display()
                );
            }
        }
    } else {
        // Non-Windows builds rely on mock camera backends.
        println!("cargo:warning=Building without Optris Connect SDK (mock camera backend enabled)");
    }
}
