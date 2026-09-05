//! Linux: an `$ORIGIN` rpath so `libcef.so`, which the browser feature puts
//! next to the binary, loads without `LD_LIBRARY_PATH`. Harmless without it.
fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed=CARGO_FEATURE_BROWSER");
    // Only with the browser feature: an RPATH outranks LD_LIBRARY_PATH for
    // every shared library, and the default build has no reason to carry it.
    if std::env::var_os("CARGO_FEATURE_BROWSER").is_some()
        && std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux")
    {
        println!("cargo::rustc-link-arg-bins=-Wl,-rpath,$ORIGIN");
        println!("cargo::rustc-link-arg-bins=-Wl,--disable-new-dtags");
    }
}
