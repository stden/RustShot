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
- [ ] ⌨️ **Custom hotkeys** — let user rebind PrintScreen and Ctrl+Shift+R via settings
- [ ] ⚙️ **Expanded settings system** — make settings fully applied at runtime, cover hotkeys/startup behavior, and eventually add a native settings UI
- [ ] 🪟 **Active window capture improvements** — refine the existing Alt+PrintScreen and tray capture flow with better UX, naming, and cross-platform support
- [ ] 🐧 **Wayland support** — Linux Wayland screenshot capture (currently X11 only)
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

- [ ] 📊 **Benchmark suite** — add `criterion` benchmarks for capture latency, BGRA conversion, overlay render
- [ ] 🎯 **Target: capture < 100ms** — from hotkey press to clipboard
- [ ] 🎯 **Target: overlay 60fps** — selection overlay render during drag
- [ ] 🎯 **Target: recording < 5% CPU** — 30fps recording CPU overhead
- [ ] 🎯 **Target: startup < 500ms** — time to tray icon visible
- [ ] 🎯 **Target: memory < 100MB** — for single 1920×1080 capture
- [ ] 🔍 **Profile high-DPI** — selection overlay may be slow on 4K displays; needs profiling

## 🧪 Testing

- [ ] 🧬 **Mutation testing** — use `cargo-mutants` to find untested logic
- [ ] 📋 **Clipboard tests** — TC-CLIP-001 through TC-CLIP-006 (mock-based clipboard verification)
- [ ] ⏱️ **Frame timing tests** — TC-FPS-001 through TC-FPS-004 (verify frame timestamp monotonicity)
- [ ] 🪟 **Integration tests on Windows CI** — TC-SESSION-003, TC-SESSION-004 (real recording lifecycle)
- [ ] 🔀 **Fuzzing** — fuzz BGRA conversion and region selection with random inputs
- [ ] 📈 **Coverage tracking** — add `cargo-tarpaulin` or `llvm-cov` to CI; target >90% on testable logic
- [ ] 🔄 **Property-based tests** — use `proptest` for BGRA↔RGBA roundtrip, region construction invariants

## 📦 Distribution

- [ ] 🍺 **Homebrew formula** — `brew install rustshot` for macOS
- [ ] 🪟 **Scoop/Winget package** — Windows package manager support
- [ ] 🐧 **AUR package** — Arch Linux package
- [ ] 📦 **Flatpak/Snap** — Linux universal package
- [ ] 🔄 **Auto-update** — check for new version on startup, prompt to download
- [ ] 💼 **Portable mode** — detect `rustshot.ini` next to binary for portable settings

## ⚙️ CI/CD

- [ ] 🚀 **Release pipeline** — GitHub Actions workflow to build and upload binaries on tag push
- [ ] 📊 **Code coverage in CI** — publish coverage reports to Codecov or Coveralls
- [ ] 📉 **Benchmark regression** — run benchmarks in CI, fail on >10% regression
- [ ] 🌍 **Cross-compilation** — build Linux/macOS/Windows binaries from one CI run
- [ ] 🌙 **Nightly builds** — automated nightly builds from main branch
