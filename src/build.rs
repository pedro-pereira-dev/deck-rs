fn main() {
    println!("cargo:rerun-if-changed=assets/SymbolsNerdFontMono-Regular.ttf");
    slint_build::compile("ui/app.slint").expect("Slint UI compilation failed");
}
