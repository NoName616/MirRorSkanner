use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let target_dir = manifest_dir.join("target").join("debug");
    let sdk_lib_dir = manifest_dir.join("..\\Connect_SDK\\Lib\\v120");

    let dll_name = "ImagerIPC2x64.dll";
    let source_path = sdk_lib_dir.join(dll_name);
    let dest_path = target_dir.join(dll_name);

    if !target_dir.exists() {
        fs::create_dir_all(&target_dir).expect("Failed to create target/debug directory");
    }

    if source_path.exists() {
        fs::copy(&source_path, &dest_path).expect("Failed to copy DLL");
        println!("Copied {} to {}", source_path.display(), dest_path.display());
    } else {
        panic!("Source DLL not found at {}", source_path.display());
    }
}