fn main() {
    // Dioxus 0.6.3 does not require build scripts for basic desktop applications
    println!("cargo:rerun-if-changed=src/");
}
