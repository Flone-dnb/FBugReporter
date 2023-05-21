use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    if cfg!(windows) {
        // Just use provided sqlite3 library.
        let sqlite3_lib_dir = env::current_dir()
            .unwrap()
            .join(["..", "sqlite3-windows"].iter().collect::<PathBuf>());

        println!(
            "cargo:rustc-link-search={}",
            sqlite3_lib_dir.to_string_lossy()
        );
    }
}
