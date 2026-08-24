fn main() {
    if let Ok(bundled) = std::env::var("MONYACODE_BUNDLE") {
        println!("cargo:rustc-env=MONYACODE_BUNDLE={}", bundled);
    }
}
