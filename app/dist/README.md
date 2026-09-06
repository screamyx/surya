# Packaging

## Linux (implemented)

```sh
scripts/package-linux.sh            # release build (thin LTO, stripped)
PROFILE=debug scripts/package-linux.sh   # fast smoke package
```

Produces `target/package/surya-<version>-linux-<arch>.tar.gz` containing:

- `surya` — the binary (headed by default; `surya headless` runs the engine alone)
- `surya.desktop` — XDG desktop entry
- `surya.png` — 1024×1024 Surya app icon
- `install.sh` — installs into `~/.local/{bin,share/applications,share/icons}`

The release profile in the root `Cargo.toml` sets `lto = "thin"` and
`strip = "symbols"` for distribution builds.

## macOS

```sh
scripts/package-macos.sh    # → target/package/surya-<version>-macos-<arch>.dmg
```

Builds the release binary, assembles `Surya.app` (Info.plist + icns), ad-hoc
signs it (set `CODESIGN_IDENTITY` for a real Developer ID), and wraps it in a
dmg. The auto-update tarball retains an internal `Surya.app` path so older
installed builds can update into Surya. CI runs this on tags
(`.github/workflows/release.yml`). The manual steps it automates, for reference
(run on a macOS host — gpui needs Metal; no cross-build from Linux):

1. Build the universal (or per-arch) binary:
   ```sh
   cargo build --release -p surya --target aarch64-apple-darwin
   cargo build --release -p surya --target x86_64-apple-darwin
   lipo -create -output surya \
     target/aarch64-apple-darwin/release/surya \
     target/x86_64-apple-darwin/release/surya
   ```
2. Assemble the bundle:
   ```sh
   mkdir -p Surya.app/Contents/{MacOS,Resources}
   cp surya Surya.app/Contents/MacOS/surya
   sed "s/__VERSION__/$(grep -m1 '^version' Cargo.toml | sed 's/.*"\(.*\)".*/\1/')/" \
     dist/macos/Info.plist > Surya.app/Contents/Info.plist
   ```
3. Icon: generate `surya.icns` from `dist/macos/icon-1024.png` (the macOS-shaped
   variant of the artwork — squircle mask, margins, and shadow pre-baked, since
   `sips` can't apply an alpha mask) and place it at
   `Surya.app/Contents/Resources/surya.icns`:
   ```sh
   mkdir surya.iconset && sips -z 256 256 dist/macos/icon-1024.png --out surya.iconset/icon_256x256.png
   iconutil -c icns surya.iconset -o Surya.app/Contents/Resources/surya.icns
   ```
4. Sign + notarize (required for distribution):
   ```sh
   codesign --deep --force --options runtime --sign "Developer ID Application: …" Surya.app
   xcrun notarytool submit Surya.zip --keychain-profile … --wait
   xcrun stapler staple Surya.app
   ```
5. Ship as a `.dmg` (`hdiutil create -volname Surya -srcfolder Surya.app -ov -format UDZO Surya.dmg`).
