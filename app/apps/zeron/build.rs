//! Linux: an `$ORIGIN` rpath so `libcef.so`, which the browser feature puts
//! next to the binary, loads without `LD_LIBRARY_PATH`. Harmless without it.
fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
        println!("cargo::rustc-link-arg-bins=-Wl,-rpath,$ORIGIN");
        println!("cargo::rustc-link-arg-bins=-Wl,--disable-new-dtags");
    }
}
