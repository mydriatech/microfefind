fn main() {
    println!(
        "cargo:rustc-env=CARGO_PKG_VERSION={}",
        std::env::var("CARGO_PKG_VERSION_OVERRIDE").unwrap_or(String::from("0.0.0"))
    );
    println!("cargo:rerun-if-env-changed=CARGO_PKG_VERSION_OVERRIDE");
}
