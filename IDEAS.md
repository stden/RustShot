# 💡 Ideas

## ✨ Features

- [ ] 🎞️ **GIF recording** — record screen region as animated GIF (lightweight alternative to MP4)
- [ ] 🎤 **Audio recording** — add microphone/system audio capture to screen recordings
- [ ] ✏️ **Annotation tools** — draw arrows, rectangles, text on screenshots before saving
- [ ] ⏱️ **Delay capture** — timer (3s/5s/10s) before taking a screenshot
- [ ] 📜 **Scrolling capture** — capture long pages by auto-scrolling and stitching
- [ ] 🔤 **OCR (text from screenshot)** — extract text from selected region using Tesseract or similar
- [ ] ☁️ **Upload to cloud** — quick share via Imgur, S3, or custom server; copy link to clipboard
- [ ] 📌 **Pin screenshot** — keep screenshot floating on top of all windows (always-on-top overlay)
- [ ] 🕐 **History panel** — show recent screenshots in tray menu or separate window
- [ ] ⌨️ **Custom hotkeys** — let users rebind capture and recording shortcuts via settings ([#9](https://github.com/stden/RustShot/issues/9))
- [ ] ⚙️ **Expanded settings system** — make settings fully applied at runtime, cover hotkeys/startup behavior, and eventually add a native settings UI
- [ ] 🪟 **Active window capture improvements** — refine the existing Alt+PrintScreen and tray capture flow with better UX, naming, and cross-platform support
- [ ] 🐧 **Wayland support** — Linux Wayland screenshot capture through desktop portals ([#10](https://github.com/stden/RustShot/issues/10))
- [ ] 🍎 **macOS tray and hotkeys** — bring tray icon and global hotkeys to macOS
- [ ] 🐧 **Linux tray and hotkeys** — bring tray icon and global hotkeys to Linux
- [ ] 🔔 **Notification on capture** — show system toast/notification after screenshot or recording is saved
- [ ] 📋 **Auto-copy to clipboard in CLI mode** — option `--clipboard` to copy screenshot to clipboard even in CLI mode
- [ ] 📂 **Multiple output formats at once** — `--format png,jpg` to save in several formats simultaneously
- [ ] 🖼️ **Thumbnail preview** — show small preview of captured screenshot before it disappears
- [ ] 🎨 **Color picker** — pick color from any pixel on screen, copy hex/rgb to clipboard

## 🏗️ Architecture

- [ ] 📦 **Library + binary split** — extract core logic into `rustshot-lib` crate for reuse by other tools
- [ ] ⚠️ **Typed errors** — replace `Result<_, String>` with a proper error enum (`RustShotError`)
- [ ] 🔌 **Plugin system** — allow extensions for custom output targets, formats, or post-processing
- [ ] ⚡ **Async recording pipeline** — use `tokio` for non-blocking frame capture and encoding
- [ ] 🎮 **GPU-accelerated overlay** — use `wgpu` instead of `softbuffer` for selection overlay rendering

## 🚀 Performance

- [x] 📊 **Benchmark suite** — Criterion covers capture, conversion, selection, recording, settings, CLI, and tray hot paths
- [ ] 🎯 **Target: capture < 100ms** — measure hotkey-to-clipboard p50/p95 latency ([#6](https://github.com/stden/RustShot/issues/6))
- [ ] 🎯 **Target: overlay 60fps** — selection overlay render during drag
- [ ] 🎯 **Target: recording < 5% CPU** — 30fps recording CPU overhead
- [ ] 🎯 **Target: startup < 500ms** — time to tray icon visible
- [ ] 🎯 **Target: memory < 100MB** — for single 1920×1080 capture
- [ ] 🔍 **Profile high-DPI** — add explicit 4K overlay redraw and memory budgets ([#7](https://github.com/stden/RustShot/issues/7))

## 🧪 Testing

- [ ] 🧬 **Mutation testing** — use `cargo-mutants` to find untested logic
- [ ] 📋 **Clipboard tests** — TC-CLIP-001 through TC-CLIP-006 (mock-based clipboard verification)
- [ ] ⏱️ **Frame timing tests** — TC-FPS-001 through TC-FPS-004 (verify frame timestamp monotonicity)
- [ ] 🪟 **Integration tests on Windows CI** — exercise the real recording lifecycle ([#8](https://github.com/stden/RustShot/issues/8))
- [ ] 🔀 **Fuzzing** — fuzz BGRA conversion and region selection with random inputs
- [x] 📈 **Coverage tracking** — Linux CI runs `cargo-tarpaulin` with a 40% coverage floor
- [ ] 📈 **Target: coverage > 90%** — raise coverage for testable non-GUI logic
- [x] 🔄 **Property-based tests** — `proptest` covers recording pixels, region invariants, and settings JSON

## 📦 Distribution

- [ ] 🍺 **Homebrew formula** — `brew install rustshot` for macOS
- [ ] 🪟 **Scoop/Winget package** — Windows package manager support
- [ ] 🐧 **AUR package** — Arch Linux package
- [ ] 📦 **Flatpak/Snap** — Linux universal package
- [ ] 🔄 **Auto-update** — check for new version on startup, prompt to download
- [ ] 💼 **Portable mode** — detect `rustshot.ini` next to binary for portable settings

## ⚙️ CI/CD

- [x] 🚀 **Release pipeline** — tagged builds publish the Windows portable binary and installer
- [ ] 📊 **Code coverage in CI** — publish coverage reports to Codecov or Coveralls
- [x] 📉 **Benchmark regression** — PR CI compares with `main` and fails confirmed regressions above 10% ([#5](https://github.com/stden/RustShot/issues/5))
- [ ] 🌍 **Cross-compilation** — build Linux/macOS/Windows binaries from one CI run
- [ ] 🌙 **Nightly builds** — automated nightly builds from main branch
