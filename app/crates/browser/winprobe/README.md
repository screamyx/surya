# winprobe

Compiles the browser crate's Windows-only modules on the MSVC target from a Linux box.
The crate itself cannot be cross-checked from Linux: gpui pulls `psm`, whose build script needs the MSVC C compiler.
This probe pulls only `futures` and `windows`, and includes the modules by path.

```
cd app/crates/browser/winprobe
cargo check --target x86_64-pc-windows-msvc
```

`rustup target add x86_64-pc-windows-msvc` once. No linker is needed for a check.
Keep the `windows` features here the same as the crate's.
