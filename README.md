<p align="center">
  <img src="https://img.shields.io/badge/🦀_Rust-powered-orange?style=for-the-badge" alt="Rust Powered"/>
  <img src="https://img.shields.io/badge/📸_Screenshots-instant-blue?style=for-the-badge" alt="Screenshots"/>
  <img src="https://img.shields.io/badge/🎬_Recording-H.264_MP4-red?style=for-the-badge" alt="Recording"/>
  <img src="https://img.shields.io/badge/⚡_Speed-blazing_fast-yellow?style=for-the-badge" alt="Speed"/>
</p>

# 📸 RustShot — *your screen, captured in a snap!*

> **One binary. Zero hassle. Screenshots and recordings done right.**

RustShot is a lightning-fast ⚡ screenshot and screen recording utility,
written in Rust 🦀 because life's too short for slow tools.

---

## 🌟 Why RustShot?

| Feature | Status | Description |
|---------|--------|-------------|
| 📷 **Screenshots** | ✅ Ready | Capture full screen or select any region |
| 🎬 **Screen Recording** | ✅ Ready | Record to MP4 (H.264) — **(Windows only)** |
| 🖱️ **Region Selection** | ✅ Ready | Click and drag — it's that easy |
| 💾 **Multiple Formats** | ✅ Ready | PNG, JPG, BMP — take your pick! |
| 🖥️ **Multi-Monitor** | ✅ Ready | Got 3 monitors? We got you covered 😎 |
| ⚡ **Tray Mode** | ✅ Ready | Sits quietly in your system tray like a good tool should |
| 🪟 **Active Window** | ✅ Ready | Grab just the focused window (Windows) |

---

## 🎯 The "I just want a screenshot" guide

```bash
# That's it. Seriously. One command.
rustshot --cli
```

🎉 Done! Your screenshot is saved. You can close this README now.

...still here? Cool, there's more! 👇

---

## 🌍 Platform support

| Platform | Screenshots | Tray Mode | Recording | Vibe |
|----------|:-----------:|:---------:|:---------:|------|
| 🪟 Windows | ✅ | ✅ | ✅ | Full power 💪 |
| 🐧 Linux | ✅ | ✅ | 🚧 | Almost there! |
| 🍎 macOS | ✅ | ✅ | 🚧 | Working on it! |

---

## 📋 Prerequisites

### 🪟 Windows & 🍎 macOS

Just have Rust installed. That's it. Go grab a ☕, you're done here.

### 🐧 Linux

X11 libraries needed — one quick command and you're golden:

<details>
<summary>🔽 Ubuntu / Debian</summary>

```bash
sudo apt-get install libxcb1-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev
```
</details>

<details>
<summary>🔽 Fedora / RHEL</summary>

```bash
sudo dnf install libxcb-devel
```
</details>

<details>
<summary>🔽 Arch Linux (btw)</summary>

```bash
sudo pacman -S libxcb
```
</details>

---

## 🔨 Building from source

```bash
cargo build              # 🐢 Debug build (for tinkering)
cargo build --release    # 🚀 Release build (for speed demons)
```

Your shiny new binary lands here:
```text
target/release/rustshot       # 🐧🍎
target/release/rustshot.exe   # 🪟
```

---

## 🚀 Usage

### 🖥️ Tray mode — *set it and forget it*

```bash
rustshot
```

Just launch it. An icon appears in your system tray. Now use hotkeys like a pro:

| 🎹 Hotkey | 🎬 What happens | 🤔 When to use |
|-----------|-----------------|----------------|
| `PrintScreen` | Select region → screenshot → clipboard 📋 | When you need *just that part* |
| `Alt+PrintScreen` | Capture active window → save PNG 🖼️ | When the whole window is the star |
| `Ctrl+Shift+R` | Start recording 🔴 | Showtime! |
| `Ctrl+Shift+R` again | Stop recording → save MP4 🎬 | And... cut! |
| Right-click tray icon | Open menu 📋 | For everything else |

**📂 Where does stuff go?**
| What | Where | Why |
|------|-------|-----|
| Region captures | 📋 Clipboard only | Quick sharing! |
| Window & CLI captures | `Pictures\RustShot\` | Auto-organized |
| Recordings | `Pictures\RustShot\recordings\` | Movie night 🍿 |

> 💡 **Tip:** On Windows, tray mode adds itself to startup — RustShot is always one hotkey away!

---

### 📸 CLI examples — *for the command-line warriors*

#### 1️⃣ Quick screenshot
```bash
rustshot --cli
# 📸 *click* — saved with an auto-generated name
```

#### 2️⃣ Custom filename
```bash
rustshot --output epic_screenshot.png
# 💾 Saved exactly where you want it
```

#### 3️⃣ Pick your format
```bash
rustshot --format jpg --output photo.jpg    # 📷 JPEG — smaller file
rustshot --format bmp --output retro.bmp    # 🖼️ BMP — for that retro feel
rustshot --format png --output crisp.png    # 🎨 PNG — pixel perfect (default)
```

#### 4️⃣ Multi-monitor magic
```bash
rustshot --list              # 👀 See all your screens
rustshot --screen 1          # 📺 Capture monitor #2
rustshot --screen 0          # 📺 Capture monitor #1 (indexing starts at 0!)
```

#### 5️⃣ Capture the active window
```bash
rustshot --window            # 🪟 Just the focused window, nothing else
```

---

## 📖 Cheat sheet

```text
🎯 BASIC
  rustshot                         → 🖥️  tray mode (recommended)
  rustshot --cli                   → 📸  quick screenshot
  rustshot --window                → 🪟  active window (Windows only)

🎨 OUTPUT
  rustshot --output file.png       → 💾  save to custom file
  rustshot --format jpg            → 🖼️  save as JPG

🖥️ SCREENS
  rustshot --list                  → 📋  list monitors
  rustshot --screen 1              → 📺  capture specific monitor
```

---

## ⚙️ All options

```text
    --cli              📸  take one screenshot and exit
-w, --window           🪟  capture the active window (Windows only)
-o, --output <OUTPUT>  💾  output file name or path
-f, --format <FORMAT>  🎨  image format: png, jpg, bmp
-s, --screen <SCREEN>  🖥️  screen number (0-indexed)
-l, --list             📋  list available screens
-h, --help             ❓  show help
-V, --version          🏷️  show version
```

---

## 🛠️ Development

```bash
cargo check            # ✅ Does it compile?
cargo test             # 🧪 Do the tests pass?
cargo fmt              # 💅 Make it pretty
cargo clippy           # 🔍 Lint it good
cargo bench            # 🏎️ How fast is it?
python wiki/lint.py    # 🧠 Validate the RAG knowledge base
```

### 📐 The TDD way of life

```text
1. ✏️  Write a test              "This is what I expect"
2. ❌  Watch it fail              "Yep, it doesn't work yet"
3. 🔧  Write the minimal fix     "Just enough to make it green"
4. ✅  Watch it pass              "Sweet success!"
5. 🧪  Run ALL tests              "Nothing else broke... right?"
```

---

## 📥 Downloads

Head over to [**Releases**](https://github.com/stden/RustShot/releases) and grab what you need:

| File | What is it | For whom |
|------|-----------|----------|
| `rustshot.exe` | 📦 Portable executable | "I don't like installers" |
| `RustShotSetup.exe` | 🔧 Windows installer | "Set it up properly" |

---

## 📚 Reference files

| File | Description |
|------|------------|
| 📘 `AGENTS.md` | General rules for AI assistants |
| 🤖 `CLAUDE.md` | Instructions for Claude |
| 💎 `GEMINI.md` | Instructions for Gemini |
| 🗺️ [docs/README.md](docs/README.md) | Documentation map & repository guide |
| 🧠 [wiki/index.md](wiki/index.md) | Retrieval knowledge base index |
| 📦 [INSTALLERS.md](docs/INSTALLERS.md) | 🎓 Tutorial: build Windows installers for Rust apps |
| 🚀 [INNO_SETUP_PIPELINE.md](docs/INNO_SETUP_PIPELINE.md) | 🎓 Lesson: Inno Setup + GitHub Actions pipeline |
| ⚙️ [github-pipeline.md](docs/github-pipeline.md) | 🎓 Guide: GitHub Actions CI/CD pipeline for Rust |

---

## 📜 License

`MIT OR Apache-2.0` — use it however you like! 🎉

---

<p align="center">
  <b>Made with 🦀 and ❤️</b><br/>
  <i>Because every screen deserves to be remembered.</i>
</p>

---

[Версия на русском языке (README.ru.md)](README.ru.md)

## Testing

- Repository-wide testing/TDD guide: `TESTING_TDD_GUIDE.md`
- Stack checklist: `TESTING_CHECKLIST_RUST.md`
