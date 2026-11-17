use std::collections::HashMap;
use std::path::PathBuf;

fn main() {
    let material_library =
        PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("material-1.1.0/material.slint");
    let config = slint_build::CompilerConfiguration::new()
        .with_library_paths(HashMap::from([("material".into(), material_library)]));
    println!("cargo:rerun-if-changed=assets/SymbolsNerdFontMono-Regular.ttf");
    slint_build::compile_with_config("ui/app.slint", config).expect("Slint UI compilation failed");
}
