fn main() {
    println!("cargo:rerun-if-changed=assets/SymbolsNerdFontMono-Regular.ttf");
    slint_build::compile("src/ui/window.slint").expect("Slint UI compilation failed");
}
