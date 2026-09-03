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

## 🎯 Tutorial, part 1 — *fresh clone to first screenshot*

Start here. Four steps, no detours. (🐧 Linux: install the packages under
[Prerequisites](#-prerequisites) first, then come back.)

**1️⃣ Get Rust and the sources.** `rust-toolchain.toml` pins the toolchain to
**1.93** and `rustup` fetches it for you. On 🪟 Windows you also need the Visual
Studio C++ build tools, because the build targets `x86_64-pc-windows-msvc`.

```bash
git clone https://github.com/stden/RustShot.git
cd RustShot
cargo build --release
```

> 🪟 Shortcut: `RustShot.cmd` in the repo root does the build, copies
> `target\release\RustShot.exe` next to itself, and launches it.

**2️⃣ Take one screenshot, right now.**

```bash
target/release/RustShot --cli        # 🐧🍎
target\release\RustShot.exe --cli    # 🪟
```

📸 The PNG lands in your Pictures folder under `RustShot\`, with a timestamped
name.

> 🪟 One catch: RustShot is built as a Windows GUI app, so it has no console. In
> **PowerShell** the command returns instantly and prints nothing. Pipe it
> (`.\RustShot.exe --cli | Out-String`) or run it from **cmd.exe** to see
> `Screenshot saved to: ...` and a real exit code.

**3️⃣ Start tray mode.** No arguments at all — that *is* tray mode:

```bash
target\release\RustShot.exe          # 🪟 (drop the .exe on 🐧🍎)
```

> 🪟 **Windows 11 hides brand-new tray icons.** Yours went into the hidden-icons
> flyout behind the `^` chevron on the taskbar — an empty tray does *not* mean
> RustShot failed to start. Click `^`, find the blue camera icon, and drag it
> down onto the taskbar to keep it there. That drag is what writes
> `IsPromoted = 1` under `HKCU\Control Panel\NotifyIconSettings\<id>`, and it is
> remembered **per exe path**, so a copy of the binary in another folder starts
> out hidden again.
>
> Nothing behind the chevron either? Then RustShot really did exit during
> startup, and the reason is the last line of
> `%APPDATA%\RustShot\rustshot.log`.

> On its first bare Windows launch, RustShot installs itself at
> `%LOCALAPPDATA%\Programs\RustShot\RustShot.exe`, registers that stable copy to
> start at sign-in, relaunches it, and exits the original process. This happens
> only when **Start with Windows** is enabled (the default); CLI, MCP, and helper
> processes never self-install.

**4️⃣ Press `PrintScreen` and drag a box.** The region goes straight to your 📋
clipboard — paste it anywhere. `Esc` or right-click cancels, and drags of 5 px or
less are discarded, so a plain click looks like nothing happened.

🎉 That's the whole loop: build, tray, capture.

> 📖 **Tutorial, part 2 → [docs/TUTORIAL.md](docs/TUTORIAL.md)** — recording, the
> settings dialog, switching auto-start off, the MCP server, and what does and
> doesn't work off Windows.

...still here? Cool, there's a reference section too! 👇

---

## 🌍 Platform support

| Platform | Screenshots | Tray + menu | Hotkeys | Recording | Vibe |
|----------|:-----------:|:-----------:|:-------:|:---------:|------|
| 🪟 Windows | ✅ | ✅ | ✅ | ✅ | Full power 💪 |
| 🐧 Linux | ✅ | ✅ ¹ | ❔ ² | ❌ | Screenshots + tray |
| 🍎 macOS | ✅ | ✅ | ⚠️ ³ | ❌ | Screenshots + tray |

¹ Needs `libayatana-appindicator3` **at runtime** — see
[Prerequisites](#-prerequisites). Without it the tray panics on startup.
² Not verified by us; tracked in `IDEAS.md`.
³ The hotkeys are `PrintScreen` / `Alt+PrintScreen` — keys Apple keyboards don't
have — and they can't be rebound.

Recording is Windows-only by design (H.264 via Media Foundation); elsewhere it
answers `Recording is only supported on Windows`. Active-window capture
(`--window`, `--window-id`, `--list-windows`) is Windows-only too. Details in
[docs/TUTORIAL.md](docs/TUTORIAL.md#-platform-reality-check).

---

## 📋 Prerequisites

### 🪟 Windows & 🍎 macOS

Install Rust via [rustup](https://rustup.rs) — `rust-toolchain.toml` pins the
toolchain to **1.93** and rustup fetches it automatically. On 🪟 Windows you also
need the Visual Studio C++ build tools, since the build targets
`x86_64-pc-windows-msvc`. 🍎 macOS needs nothing else. Go grab a ☕.

### 🐧 Linux

X11 **and** GTK libraries needed — the tray reaches the desktop through
appindicator, so it's more than just xcb:

<details>
<summary>🔽 Ubuntu / Debian</summary>

```bash
# capture path
sudo apt-get install pkg-config libxcb1-dev libxcb-render0-dev \
  libxcb-shape0-dev libxcb-xfixes0-dev
# ...plus these for the tray
sudo apt-get install libglib2.0-dev libgtk-3-dev libxdo-dev libatk1.0-dev \
  libgdk-pixbuf2.0-dev libcairo2-dev libpango1.0-dev libayatana-appindicator3-dev
```

`libayatana-appindicator3-dev` is a **runtime** requirement, not just a build
one: `tray-icon` reaches the Linux tray through `libappindicator-sys`, which
`dlopen`s `ayatana-appindicator3` (or `appindicator3`) and **panics** when
neither is there — so the tray-mode process dies rather than logging an error.
This is the exact set CI installs; the `Install X11 and GTK dependencies` step in
[`.github/workflows/ci.yml`](.github/workflows/ci.yml) is the authoritative list.

```bash
# (nothing more to run here — see above)
```
</details>

<details>
<summary>🔽 Fedora / RHEL</summary>

```bash
sudo dnf install libxcb-devel
# ...plus the GTK 3, libxdo, glib, atk, gdk-pixbuf, cairo, pango and
# ayatana-appindicator3 development packages.
```

We don't list exact Fedora package names, because we'd be guessing. Match the
Ubuntu set in [`.github/workflows/ci.yml`](.github/workflows/ci.yml).
</details>

<details>
<summary>🔽 Arch Linux (btw)</summary>

```bash
sudo pacman -S libxcb
# ...plus gtk3, xdotool, glib2, atk, gdk-pixbuf2, cairo, pango and an
# appindicator implementation.
```

Same caveat as Fedora: [`.github/workflows/ci.yml`](.github/workflows/ci.yml)
holds the authoritative (Ubuntu-named) list.
</details>

---

## 🔨 Building from source

```bash
cargo build              # 🐢 Debug build (for tinkering)
cargo build --release    # 🚀 Release build (for speed demons)
```

Your shiny new binary lands here:
```text
target/release/RustShot       # 🐧🍎
target/release/RustShot.exe   # 🪟
```

---

## 🚀 Usage

### 🖥️ Tray mode — *set it and forget it*

```bash
RustShot
```

Just launch it — no arguments. On 🪟 Windows 11 the new icon starts out **hidden**
behind the taskbar's `^` chevron; drag it out once to pin it (see
[Tutorial, part 1](#-tutorial-part-1--fresh-clone-to-first-screenshot)). Then use
hotkeys like a pro:

| 🎹 Hotkey | 🎬 What happens | 🤔 When to use |
|-----------|-----------------|----------------|
| `PrintScreen` | Select region → screenshot → clipboard 📋 | When you need *just that part* |
| `Alt+PrintScreen` | Capture active window → a **file** in the screenshots folder, in the format chosen in Settings (PNG by default) 🖼️ — the clipboard is left alone, unlike Windows' own Alt+PrintScreen. Windows only | When the whole window is the star |
| `Ctrl+Shift+R` | Select a region → start recording 🔴 (Windows only; the region must fit inside a single monitor) | Showtime! |
| `Ctrl+Shift+R` again | Stop recording → MP4 🎬 | And... cut! |
| Right-click tray icon | Menu: *Capture Screenshot*, *Capture Active Window*, *Open Screenshots Folder*, *Start or Stop Recording*, *Settings*, *Exit* 📋 | For everything else |

**📂 Where does stuff go?** (these are the built-in defaults; all of them are
editable in the tray's Settings dialog — see
[docs/TUTORIAL.md](docs/TUTORIAL.md#-step-7--the-settings-dialog))
| What | Where | Why |
|------|-------|-----|
| Region captures | 📋 Clipboard only — never a file | Quick sharing! |
| Window captures, and CLI captures **without** `--output` | `Pictures\RustShot\` | Auto-organized |
| CLI captures **with** `--output <path>` | exactly that path — a bare filename lands in the **current directory** | You asked for it |
| Recordings | `Pictures\RustShot\recordings\` | Movie night 🍿 |

> 💡 **Tip:** on 🪟 Windows, the first bare tray launch installs
> `%LOCALAPPDATA%\Programs\RustShot\RustShot.exe`; later external launches hand
> off to that existing copy without replacing it. RustShot registers the stable
> path under `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`. It's on by
> default and user-scoped (`HKCU` only, so no admin rights). To switch it off —
> and where `settings.json` lives — see
> [docs/TUTORIAL.md](docs/TUTORIAL.md#-step-8--auto-start-and-where-settings-live).

---

### 📸 CLI examples — *for the command-line warriors*

**The routing rule:** any capture-related flag implies one-shot CLI mode, so
`--cli` is only needed when you pass nothing else. `--cli`, `--list`, `--window`,
`--output`, `--screen`, an explicit `--format`, `--list-windows` and
`--window-id` each select CLI mode on their own; `--mcp` wins over all of them;
and only a bare command line reaches tray mode. (Yes — `--format jpg` alone
silently suppresses the tray.)

#### 1️⃣ Quick screenshot
```bash
RustShot --cli
# 📸 *click* — saved with an auto-generated name
```

#### 2️⃣ Custom filename
```bash
RustShot --output epic_screenshot.png
# 💾 Saved exactly there — a bare filename means the current directory
```

#### 3️⃣ Pick your format
```bash
RustShot --format jpg --output photo.jpg    # 📷 JPEG — smaller file
RustShot --format bmp --output retro.bmp    # 🖼️ BMP — for that retro feel
RustShot --format png --output crisp.png    # 🎨 PNG — pixel perfect (default)
```

#### 4️⃣ Multi-monitor magic
```bash
RustShot --list              # 👀 See all your screens
RustShot --screen 1          # 📺 Capture monitor #2
RustShot --screen 0          # 📺 Capture monitor #1 (indexing starts at 0!)
```

#### 5️⃣ Capture the active window
```bash
RustShot --window            # 🪟 Just the focused window (Windows only)
```

#### 6️⃣ Pick a specific window
```bash
RustShot --list-windows      # 🪟 List open windows as "<id>  <title>" (Windows only)
RustShot --window-id 262788  # 🎯 Capture that exact window by its id (Windows only)
```

---

## 🤖 MCP server — *let an AI take the screenshots*

RustShot speaks the [Model Context Protocol](https://modelcontextprotocol.io):
point an MCP client (an AI assistant, an editor) at it and it can list and
capture screens and windows for you.

```bash
RustShot --mcp               # 🔌 Serve MCP over stdio (JSON-RPC)
```

Example client config:

```json
{
  "mcpServers": {
    "rustshot": { "command": "RustShot.exe", "args": ["--mcp"] }
  }
}
```

Tools exposed: `list_screens`, `list_windows`, `capture_screen`, and
`capture_window`. The capture tools save a file **and** return the image inline,
so the assistant can view the screenshot directly.

---

## 📖 Cheat sheet

```text
🎯 BASIC
  RustShot                         → 🖥️  tray mode (recommended)
  RustShot --cli                   → 📸  quick screenshot
  RustShot --window                → 🪟  active window (Windows only)

🎨 OUTPUT
  RustShot --output file.png       → 💾  save to custom file
  RustShot --format jpg            → 🖼️  save as JPG

🖥️ SCREENS
  RustShot --list                  → 📋  list monitors
  RustShot --screen 1              → 📺  capture specific monitor
```

---

## ⚙️ All options

```text
    --cli              📸  take one screenshot and exit
    --mcp              🤖  run as an MCP server over stdio
-w, --window           🪟  capture the active window (Windows only)
    --list-windows     🪟  list open windows (id and title)
    --window-id <ID>   🎯  capture a specific window by id (Windows only)
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
cargo check    # ✅ Does it compile?
cargo test     # 🧪 Do the tests pass? (unit + integration + doc + BDD + lint policy)
cargo fmt      # 💅 Make it pretty
cargo lint     # 🔍 Lint it good — exactly what CI runs
cargo bench    # 🏎️ How fast is it?
```

### ⚡ Performance benchmarks

The Criterion suite includes the hot paths for capture, selection, recording,
settings, CLI routing, and tray rendering. To measure the selection overlay's
1920×1080 cursor redraw path in isolation:

```bash
cargo bench --bench benchmarks -- overlay_redraw/1920x1080_crosshair

# ...and the deterministic, CI-gated twin of the same path:
cargo bench --bench performance_gate -- selection_overlay_redraw_1080p
```

Reach for the `performance_gate` target when judging a change: it pins its
sampling configuration, its ids own the recorded baselines, and its ceilings live
in `scripts/benchmark_guard.ps1`. The `benchmarks` target is exploratory and also
probes live screen capture, which can stall on a headless or busy session.

Caching the immutable screenshot background is what makes this redraw
*incremental* instead of a full-frame recomposition. Measured on one Windows
development machine (2026-08-26, single run — read it as an order of magnitude,
not a spec): `overlay_redraw/1920x1080_crosshair` at **13.5 µs**
[12.9 µs, 14.2 µs], against **2.19 ms** [2.14 ms, 2.26 ms] to rebuild that
background from scratch (`performance_gate/selection_overlay_background_1080p`) —
the cost the uncached path paid on every cursor move. Absolute timings depend on
the machine, so use Criterion's before/after comparison when evaluating a change;
the ceiling CI actually enforces is **0.2 ms**.

The cache stores one `u32` pixel buffer for the selection session: about 8 MiB
at 1920×1080 and 32 MiB at 3840×2160. It is rebuilt only when the overlay or
captured image dimensions change. Cursor and drag updates restore only the
rectangles the *previous* crosshair or border covered from the cached background,
paint the new ones, and hand those rectangles to `present_with_damage` — a
whole-frame copy happens only on the first frame, or when `softbuffer` can't
vouch for the buffer's previous contents.

The 4K BGRA→RGBA channel swap also uses AVX2 when available, with a portable
scalar fallback for older and non-x86 processors. On the same machine and run,
`performance_gate/bgra_to_rgba_4k` measured **1.61 ms** [1.54 ms, 1.69 ms]. The
scalar-versus-AVX2 delta isn't benchmarked — `PixelSwizzle` is crate-private and
`detect` always takes AVX2 where the CPU offers it — so treat the vectorised path
as guarded by that benchmark's **4 ms** budget rather than by a quoted speedup.

Pull requests run `cargo bench --bench performance_gate` on Windows. Once the
benchmark target exists on the base branch, CI measures base and head on the
same runner. `scripts/benchmark_guard.ps1` can then fail a pull request two ways:
an **absolute** per-benchmark budget is exceeded when the *upper* bound of the
median's confidence interval crosses its ceiling, and a **relative** regression
is confirmed when the *lower* bound of the median change exceeds 10%. Absolute
budgets are checked on every run; a newly added benchmark has no base result, so
`--baseline-lenient` skips its relative check for one run and the budget is all
that applies. The Criterion HTML and JSON for the `performance_gate` group is
uploaded as a workflow artifact, kept for 14 days.

> **Fun fact:** `cargo test` runs the Gherkin features in `tests/features/` as
> part of the suite 🧪 — the behavioural spec is executable, not a document
> sitting next to the code. Roughly 900 tests and 200-odd scenarios at the time of
> writing; run `cargo test` for today's number rather than trusting this sentence,
> which is exactly how the previous "2700+" claim ended up three times too high.

---

## 📥 Downloads

Head over to [**Releases**](https://github.com/stden/RustShot/releases) and grab what you need:

| File | What is it | For whom |
|------|-----------|----------|
| `rustshot.exe` | 📦 Single executable; a bare tray launch self-installs when auto-start is enabled | "I don't like setup wizards" |
| `RustShotSetup.exe` | 🔧 Windows installer | "Set it up properly" |

---

## 📚 Reference files

| File | Description |
|------|------------|
| 📘 `AGENTS.md` | General rules for AI assistants |
| 🤖 `CLAUDE.md` | Instructions for Claude |
| 💎 `GEMINI.md` | Instructions for Gemini |
| 📦 [INSTALLERS.md](docs/INSTALLERS.md) | 🎓 Tutorial: build Windows installers for Rust apps |
| 🚀 [INNO_SETUP_PIPELINE.md](docs/INNO_SETUP_PIPELINE.md) | 🎓 Lesson: Inno Setup + GitHub Actions pipeline |
| 📖 [docs/TUTORIAL.md](docs/TUTORIAL.md) | 🎓 **Tutorial, part 2** — recording, settings, auto-start, MCP, platform notes |
| 🗺️ [docs/README.md](docs/README.md) | **Documentation map** — architecture (rustdoc, start at `src/lib.rs`), executable specs, process docs |

---

## 📜 License

`MIT OR Apache-2.0` — use it however you like! 🎉

---

<p align="center">
  <b>Made with 🦀 and ❤️</b><br/>
  <i>Because every screen deserves to be remembered.</i>
</p>

---

## Testing

- Executable specifications (BDD): Gherkin features in `tests/features/`, run with
  `cargo test --test bdd`. These are the behavioural spec.
- General testing/TDD guidance is not duplicated here — it lives in the central
  `iopm` repo (`docs/TESTING_TDD_GUIDE.md`, `docs/TESTING_CHECKLIST_RUST.md`).
