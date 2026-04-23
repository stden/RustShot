// RustShot — Product Guide & Technical Overview
// Canonical Typst source for repository documentation.

#set page(
  paper: "a4",
  margin: (x: 20mm, y: 22mm),
  header: context {
    if counter(page).get().first() > 1 [
      #set text(size: 9pt, fill: gray.darken(25%))
      🦀 RustShot v0.1.0
      #h(1fr)
      Product Guide & Technical Overview
      #h(1fr)
      #counter(page).display()
    ]
  },
  footer: [],
)

#set text(
  font: "New Computer Modern",
  size: 10.8pt,
  lang: "en",
)

#set par(justify: true, leading: 0.62em)
#set heading(numbering: "1.1")

// Color palette
#let rust-orange = rgb("#F74C00")
#let rust-dark = rgb("#2B1400")
#let teal = rgb("#0D9488")
#let indigo = rgb("#4F46E5")
#let emerald = rgb("#059669")
#let rose = rgb("#E11D48")
#let amber = rgb("#D97706")
#let sky = rgb("#0284C7")
#let violet = rgb("#7C3AED")
#let slate = rgb("#475569")

#let app = "RustShot"

// Colored box helper
#let colorbox(body, color: teal, title: none) = {
  block(
    width: 100%,
    inset: 12pt,
    radius: 6pt,
    fill: color.lighten(92%),
    stroke: 1.5pt + color.lighten(50%),
  )[
    #if title != none [
      #text(weight: "bold", fill: color)[#title]
      #v(4pt)
    ]
    #body
  ]
}

// Diagram node helper
#let node(label, color: indigo, icon: "📦") = {
  box(
    inset: (x: 10pt, y: 6pt),
    radius: 8pt,
    fill: color.lighten(88%),
    stroke: 1.2pt + color.lighten(40%),
  )[#text(fill: color.darken(20%), weight: "bold")[#icon #label]]
}

// Arrow helper
#let arrow(color: slate) = {
  h(4pt)
  text(fill: color, size: 14pt)[→]
  h(4pt)
}

// ─────────────────────────────────────────────
// Title page
// ─────────────────────────────────────────────

#align(center + horizon)[
  #v(2em)
  #block(
    inset: 24pt,
    radius: 16pt,
    fill: rust-orange.lighten(94%),
    stroke: 2pt + rust-orange.lighten(60%),
  )[
    #text(size: 48pt)[🦀]
    #v(0.3em)
    #text(size: 36pt, weight: "bold", fill: rust-orange)[#app]
  ]
  #v(0.6em)
  #line(length: 58%, stroke: 2pt + rust-orange.lighten(60%))
  #v(0.7em)
  #text(size: 16pt, fill: slate)[📸 Screenshot and 🎬 Screen Recording Utility]
  #v(0.8em)
  #text(size: 12pt, fill: indigo)[Product Guide & Technical Overview]
  #v(2em)
  #block(
    inset: 14pt,
    radius: 8pt,
    fill: indigo.lighten(95%),
    stroke: 1pt + indigo.lighten(70%),
  )[
    #text(size: 11pt, fill: indigo)[📋 Version 0.1.0] #h(1.5em)
    #text(size: 11pt, fill: emerald)[🔓 MIT OR Apache-2.0] #h(1.5em)
    #text(size: 11pt, fill: teal)[🏗️ Built with Rust]
  ]
  #v(2em)
  #text(size: 10pt, fill: gray.darken(10%))[📂 Repository: `stden/RustShot`]
  #v(3em)
  #text(size: 9.5pt, fill: gray)[⚙️ Generated from `RustShot.typ`]
]

#pagebreak()

#outline(title: [📑 Contents], depth: 3)

#pagebreak()

= 🎯 Product Summary

== 📌 Purpose

#colorbox(color: teal)[
  #app is a compact desktop utility for fast screenshots and short recordings.
  It is designed around a direct workflow: trigger a hotkey or CLI command, capture the relevant screen content, and continue working without opening a large editor or media suite.
]

The project combines three practical roles in one binary:

#colorbox(color: indigo, title: "🔧 Three Modes in One Binary")[
  - 🖥️ *Command-line interface* — one-shot capture and scripting
  - 🔔 *Tray application* — global hotkeys and quick desktop access
  - 🎛️ *Internal helper modes* — selection overlays and recording setup
]

The implementation favors a narrow, dependable feature set over a large UI surface area.
That bias shows up throughout the codebase: small helper functions, explicit path handling, simple JSON persistence, and native platform APIs where they make the tool more predictable.

== ✨ Main Capabilities

#figure(
  block(
    width: 100%,
    inset: 12pt,
    radius: 8pt,
    fill: emerald.lighten(94%),
    stroke: 1.2pt + emerald.lighten(50%),
  )[
    #grid(
      columns: (1fr, 1fr),
      gutter: 8pt,
      [📸 Screenshot capture in `png`, `jpg`, `bmp`],
      [🎯 Region selection via overlay workflow],
      [🪟 Active-window capture on Windows],
      [🖥️ Multi-monitor support],
      [🎬 Screen recording to H.264 MP4 (Win)],
      [📋 Clipboard copy for tray region shots],
      [🔔 System tray with global hotkeys],
      [⚙️ Persistent settings for dirs, format, FPS],
    )
  ],
  caption: [Main capabilities overview],
)

= 🔄 User Workflows

== 🔔 Tray Mode

Launching `rustshot` with no arguments starts tray mode.
This is the default interactive experience and is intended for daily desktop use.

// Tray workflow diagram
#figure(
  block(
    width: 100%,
    inset: 14pt,
    radius: 10pt,
    fill: sky.lighten(95%),
    stroke: 1.5pt + sky.lighten(55%),
  )[
    #align(center)[
      #text(fill: sky, weight: "bold", size: 11pt)[🔄 Tray Workflow Diagram]
      #v(8pt)
      #node("Launch", color: emerald, icon: "🚀")
      #arrow(color: emerald)
      #node("Tray Icon", color: indigo, icon: "🔔")
      #arrow(color: indigo)
      #node("Hotkey / Menu", color: amber, icon: "⌨️")
      #v(10pt)
      #h(4em)
      #node("Capture", color: teal, icon: "📸")
      #arrow(color: teal)
      #node("Save / Clipboard", color: emerald, icon: "💾")
    ]
  ],
  caption: [Tray mode lifecycle],
)

#v(8pt)

#figure(
  table(
    columns: (auto, 1fr),
    inset: 8pt,
    align: (left, left),
    fill: (col, row) => if row == 0 { indigo.lighten(88%) } else if calc.odd(row) { indigo.lighten(96%) } else { white },
    [*⌨️ Action*], [*📋 Result*],
    [🖨️ PrintScreen], [Open the selection overlay and copy the selected region to the clipboard],
    [🪟 Alt+PrintScreen], [Capture the active window and save to screenshots directory (Windows)],
    [🎬 Ctrl+Shift+R], [Start or stop region recording (Windows)],
    [📸 Tray → Capture Screenshot], [Same as PrintScreen hotkey],
    [🪟 Tray → Capture Active Window], [Same as Alt+PrintScreen hotkey (Windows)],
    [📂 Tray → Open Screenshots Folder], [Open the configured screenshots directory],
    [🎬 Tray → Start/Stop Recording], [Same as Ctrl+Shift+R hotkey (Windows)],
    [⚙️ Tray → Settings], [Open settings dialog (native on Windows, editor on other platforms)],
    [🚪 Tray → Exit], [Stop the tray application],
  ),
  caption: [Default tray workflow],
)

Tray mode is implemented as a long-lived event loop.
It owns the tray icon, global hotkey registration, and helper subprocess management.

== 🖥️ CLI Mode

CLI mode is used for direct capture commands and scripting.
It becomes active when the binary is started with `--cli` or with other capture-related flags such as `--list`, `--screen`, `--output`, `--format`, or `--window`.

#colorbox(color: violet, title: "💻 Typical Examples")[
  ```bash
  rustshot --cli
  rustshot --list
  rustshot --screen 1 --output monitor2.png
  rustshot --window
  rustshot --format jpg --output shot.jpg
  ```
]

CLI capture writes the output file to either:

- the user-specified `--output` path, or
- the configured screenshots directory with an auto-generated timestamped file name

== 🎛️ Selection and Recording Helper Modes

The binary also contains hidden helper modes used internally by tray mode:

- `--select` for region screenshot selection
- `--select-record` for region recording selection

#colorbox(color: amber, title: "💡 Design Decision")[
  These modes keep short-lived overlay behavior separate from the long-running tray process.
  That separation reduces shared state and makes error handling easier to reason about.
]

= 📖 Command-Line Reference

#figure(
  block(
    width: 100%,
    inset: 12pt,
    radius: 8pt,
    fill: slate.lighten(94%),
    stroke: 1.2pt + slate.lighten(50%),
  )[
    ```text
    rustshot [OPTIONS]

    Options:
      --cli               Run in CLI mode (single capture, then exit)
      -w, --window        Capture the active window
      -o, --output <PATH> Output file path
      -f, --format <FMT>  Image format: png, jpg, bmp
      -s, --screen <IDX>  Screen index (0-based)
      -l, --list          List available screens
      -h, --help          Print help
      -V, --version       Print version
    ```
  ],
  caption: [CLI options reference],
)

== 📝 Common Examples

```bash
# 📸 Capture the default screen as PNG
rustshot --cli

# 💾 Save to an explicit file
rustshot --output my_screenshot.png

# 🖥️ Capture screen index 1 as JPEG
rustshot --screen 1 --format jpg --output monitor2.jpg

# 🪟 Capture the active window (Windows only)
rustshot --window

# 📋 List available screens
rustshot --list
```

= 📷 Capture and Recording Behavior

== 📸 Screenshots

RustShot supports several screenshot flows:

#colorbox(color: teal, title: "📸 Screenshot Flows")[
  - 🔔 Tray selection capture via `PrintScreen`
  - 🪟 Active-window capture via `Alt+PrintScreen` on Windows
  - 🖥️ CLI-based capture of a specific screen or the default screen
]

The region-selection tray workflow copies the cropped image to the clipboard.
CLI captures and active-window captures save files to disk.

When the output path is not explicitly provided, RustShot generates names in the form:

```text
screenshot_YYYYMMDD_HHMMSS_NNNNNNNNN.<ext>
```

The extra nanosecond suffix reduces collisions during repeated captures.

== 🎬 Screen Recording

Recording is currently Windows-only.

// Recording pipeline diagram
#figure(
  block(
    width: 100%,
    inset: 14pt,
    radius: 10pt,
    fill: rose.lighten(95%),
    stroke: 1.5pt + rose.lighten(55%),
  )[
    #align(center)[
      #text(fill: rose, weight: "bold", size: 11pt)[🎬 Recording Pipeline]
      #v(8pt)
      #node("Hotkey", color: amber, icon: "⌨️")
      #arrow(color: amber)
      #node("Select Region", color: indigo, icon: "🎯")
      #arrow(color: indigo)
      #node("Capture Frames", color: teal, icon: "🖼️")
      #v(10pt)
      #h(4em)
      #node("H.264 Encode", color: rose, icon: "⚡")
      #arrow(color: rose)
      #node("MP4 Output", color: emerald, icon: "💾")
    ]
  ],
  caption: [Recording pipeline flow],
)

#v(6pt)

The workflow steps:

+ Start recording with `Ctrl+Shift+R` or the tray menu
+ Drag a region in the selection overlay
+ RustShot captures frames for that region in a background thread
+ Stop recording with the same hotkey
+ The MP4 file is finalized under the recordings directory

Recordings are encoded as H.264 in an MP4 container through Windows Media Foundation.
This keeps the output broadly compatible with common players and sharing workflows while avoiding a large external media stack.

== 📂 Output Directories

#figure(
  table(
    columns: (auto, 1fr),
    inset: 8pt,
    align: (left, left),
    fill: (col, row) => if row == 0 { teal.lighten(88%) } else if calc.odd(row) { teal.lighten(96%) } else { white },
    [*📁 Type*], [*📍 Location*],
    [📸 Screenshots], [`Pictures/RustShot`],
    [🎬 Recordings], [`Pictures/RustShot/recordings`],
    [⚙️ Settings], [config dir / `RustShot/settings.json`],
    [📜 Logs], [config dir / `RustShot/rustshot.log`],
  ),
  caption: [Default output directories],
)

= ⚙️ Settings

== 📋 Persisted Fields

#figure(
  table(
    columns: (auto, auto, 1fr),
    inset: 8pt,
    align: (left, left, left),
    fill: (col, row) => if row == 0 { violet.lighten(88%) } else if calc.odd(row) { violet.lighten(96%) } else { white },
    [*🔑 Field*], [*📌 Default*], [*📝 Description*],
    [`screenshots_dir`], [`<Pictures>/RustShot`], [Directory used for saved screenshots],
    [`recordings_dir`], [`<Pictures>/RustShot/recordings`], [Directory used for MP4 recordings],
    [`default_format`], [`"png"`], [Default CLI and active-window screenshot format],
    [`recording_fps`], [`30`], [Recording frame rate],
    [`auto_start`], [`true`], [Whether RustShot registers for Windows login auto-start],
  ),
  caption: [⚙️ Persisted settings],
)

== 💾 Storage Format

Settings are stored as JSON text, but the project intentionally uses a small manual parser instead of `serde`.
That keeps the dependency surface small and lets the code handle malformed or partial input explicitly.

#colorbox(color: amber, title: "🛡️ Invalid Value Handling")[
  - ❌ Unknown `default_format` → falls back to `"png"`
  - ❌ `recording_fps` of `0` → falls back to `30`
  - ❌ Missing keys → keep their defaults
  - ❌ Unreadable/missing file → full default configuration
]

== 🖥️ Settings UI

- 🪟 On Windows, the tray menu opens a native Win32 settings dialog
- 🐧 On Linux and macOS, the settings file is opened in the default editor

The Windows dialog allows browsing for directories, changing the default format, editing FPS, and toggling auto-start.

= 🌍 Platform Support

#figure(
  table(
    columns: (auto, auto, auto, auto),
    inset: 8pt,
    align: (left, center, center, center),
    fill: (col, row) => if row == 0 { indigo.lighten(88%) } else if calc.odd(row) { indigo.lighten(96%) } else { white },
    [*🔧 Feature*], [*🪟 Windows*], [*🐧 Linux*], [*🍎 macOS*],
    [CLI screenshots], [✅], [✅], [✅],
    [Tray mode], [✅], [✅], [✅],
    [Clipboard region capture], [✅], [✅], [✅],
    [Active-window capture], [✅], [❌], [❌],
    [Screen recording], [✅], [❌], [❌],
    [Auto-start integration], [✅], [❌], [❌],
  ),
  caption: [🌍 Feature support by platform],
)

Windows has the broadest feature set because recording, active-window capture, and auto-start rely on native Windows APIs.
Linux currently depends on X11 libraries for the screenshot stack used by the project.

= 🏗️ Architecture Overview

== 📐 High-Level Layout

// Architecture diagram
#figure(
  block(
    width: 100%,
    inset: 16pt,
    radius: 12pt,
    fill: indigo.lighten(96%),
    stroke: 1.5pt + indigo.lighten(55%),
  )[
    #align(center)[
      #text(fill: indigo, weight: "bold", size: 12pt)[🏗️ Module Architecture]
      #v(12pt)

      // Top layer
      #node("main.rs", color: rust-orange, icon: "🚀")
      #v(8pt)

      // Middle layer
      #grid(
        columns: (1fr, 1fr, 1fr),
        gutter: 8pt,
        align(center)[#node("capture.rs", color: teal, icon: "📸")],
        align(center)[#node("selection.rs", color: indigo, icon: "🎯")],
        align(center)[#node("record.rs", color: rose, icon: "🎬")],
      )
      #v(8pt)

      // Bottom layer
      #grid(
        columns: (1fr, 1fr, 1fr),
        gutter: 8pt,
        align(center)[#node("settings.rs", color: violet, icon: "⚙️")],
        align(center)[#node("tray.rs", color: emerald, icon: "🔔")],
        align(center)[#node("settings_dialog.rs", color: amber, icon: "🖥️")],
      )
      #v(8pt)

      #node("lib.rs", color: slate, icon: "📦")
    ]
  ],
  caption: [Module dependency diagram],
)

== 🚀 `src/main.rs`

`main.rs` is the orchestration layer.
It parses arguments, selects the application mode, initializes logging, sets up Windows DPI awareness, starts tray mode, and launches helper subprocesses for selection and recording.

This file also contains the CLI flow, clipboard transfer for tray region screenshots, and autorun synchronization on Windows.

== 📸 `src/capture.rs`

This module handles screenshot capture and image saving.
Its responsibilities include:

- 🖥️ Screen index resolution
- 📂 Default output path generation
- 📁 Parent-directory creation
- 🖼️ Image encoding to PNG, JPEG, or BMP
- 🪟 Windows active-window capture
- 🧪 Small testable helpers for pixel conversion and benchmark support

It uses an injected internal pipeline for testing so capture logic can be verified without real hardware for every unit test.

== 🎯 `src/selection.rs`

This is the most layout-sensitive module in the project.
It is responsible for:

- 🖼️ Selection overlay behavior
- 🖱️ Mapping drag gestures to `Region`
- 🖥️ Identifying the preferred monitor
- 📐 DPI-aware logical-to-physical layout reasoning
- 📸 Helper capture flows for screenshot and recording selection

The complexity here comes from real multi-monitor coordinate systems rather than from UI visuals alone.

== 🎬 `src/record.rs`

This module contains the Windows recording implementation.
It validates the requested FPS, starts a background recording thread, configures Media Foundation, captures frames, converts RGBA data into BGRA bottom-up format, writes samples, and finalizes the MP4 output.

#colorbox(color: rose, title: "🔧 Key Helper Functions")[
  - `frame_duration_hns` — frame timing in 100ns units
  - `round_to_even` — H.264 dimension alignment
  - `pack_u32_pair` — attribute packing
  - `rgba_to_bgra_bottom_up` — pixel format conversion
  - `validate_capture_region` — region validation
]

== ⚙️ `src/settings.rs` and `src/settings_dialog.rs`

`settings.rs` owns persistence, normalization, and OS-specific path helpers.
`settings_dialog.rs` provides the Windows-only native dialog for editing settings without hand-editing JSON.

Together, these modules keep configuration simple but still practical for end users.

== 🔔 `src/tray.rs`

The tray module builds the application icon and menu structure.
It exposes stable string IDs for menu actions and constructs the programmatic 32×32 RGBA tray icon used by the running application.

= 🛡️ Error Handling and Diagnostics

#colorbox(color: rose, title: "⚠️ Error Strategy")[
  RustShot uses `Result<T, String>` consistently across the project.
  This keeps error propagation direct and user-facing messages readable.
]

Examples of boundary errors handled explicitly include:

- 🖥️ Screen enumeration failures
- 📸 Capture failures
- 📁 Directory creation failures
- 💾 File write errors
- 📋 Clipboard access issues
- 🪟 COM and Media Foundation failures on Windows

Logging is initialized early and written to the per-user config area.
That is especially useful for tray mode and child-process failures where there is no persistent terminal session.

= 🧪 Build, Test, and Benchmark Workflow

== ⚡ Common Commands

#figure(
  block(
    width: 100%,
    inset: 12pt,
    radius: 8pt,
    fill: emerald.lighten(94%),
    stroke: 1.2pt + emerald.lighten(50%),
  )[
    ```bash
    cargo check          # ✅ Verify compilation
    cargo test           # 🧪 Run all tests
    cargo fmt            # 🎨 Format code
    cargo clippy         # 🔍 Lint
    cargo build --release # 📦 Release build
    cargo bench          # ⚡ Run benchmarks
    ```
  ],
  caption: [Development commands],
)

== 📊 Verification Expectations

The repository follows a test-first development style and emphasizes small, focused tests.

// Test coverage diagram
#figure(
  block(
    width: 100%,
    inset: 14pt,
    radius: 10pt,
    fill: emerald.lighten(95%),
    stroke: 1.5pt + emerald.lighten(55%),
  )[
    #align(center)[
      #text(fill: emerald, weight: "bold", size: 11pt)[🧪 Test Coverage Areas]
      #v(10pt)
      #grid(
        columns: (1fr, 1fr),
        gutter: 10pt,
        colorbox(color: teal)[📂 Path generation & normalization],
        colorbox(color: indigo)[⚙️ Settings parsing & malformed input],
        colorbox(color: violet)[🖥️ Screen & monitor index resolution],
        colorbox(color: amber)[📐 DPI & region geometry helpers],
        colorbox(color: rose)[🎬 Recording timing & pixel conversion],
        colorbox(color: emerald)[🔔 Tray icon pixels & menu constants],
      )
    ]
  ],
  caption: [Test suite coverage map — 2,300+ automated tests],
)

Benchmarks cover helpers that matter in repeated desktop workflows, such as pixel conversion, image preparation, JSON parsing helpers, and coordinate logic.

= 📦 Release and Packaging Notes

The repository includes installer and release-pipeline documentation alongside the application code.
That reflects the project's scope: a desktop tool is only useful when it is buildable, installable, and supportable.

#colorbox(color: sky, title: "📚 Supporting Materials")[
  - 📦 Installer guidance
  - 🔧 Inno Setup documentation
  - 🚀 Release pipeline notes
  - ⚡ Benchmark and testing artifacts
  - 📋 Project plans and review documents
]

= ⚠️ Practical Constraints

The most sensitive engineering areas are:

#figure(
  block(
    width: 100%,
    inset: 14pt,
    radius: 10pt,
    fill: amber.lighten(94%),
    stroke: 1.5pt + amber.lighten(50%),
  )[
    #grid(
      columns: (1fr, 1fr),
      gutter: 10pt,
      colorbox(color: rose)[🖥️ Multi-monitor layout and DPI translation],
      colorbox(color: amber)[🎬 Windows recording object lifetime and finalization],
      colorbox(color: violet)[🪟 Platform-specific desktop integration],
      colorbox(color: indigo)[📋 Clipboard behavior in helper subprocesses],
    )
    #v(6pt)
    #align(center)[
      #colorbox(color: teal)[🔔 Keeping the tray workflow responsive while spawning short-lived helpers]
    ]
  ],
  caption: [⚠️ High-risk engineering areas],
)

These are the areas where subtle regressions are most likely, even if the visible feature appears simple.

= 🏁 Conclusion

#colorbox(color: rust-orange, title: "🦀 What RustShot Is")[
  RustShot is intentionally narrow in scope.
  It is not a full image editor or video suite.
  Its value comes from doing a small set of capture tasks quickly, predictably, and with enough native integration to feel practical on a real desktop.
]

That combination makes the project useful in two ways:

#grid(
  columns: (1fr, 1fr),
  gutter: 12pt,
  colorbox(color: emerald, title: "👤 For End Users")[
    A fast, lightweight utility for screenshots and recordings that stays out of the way.
  ],
  colorbox(color: indigo, title: "🦀 For Developers")[
    A compact example of real-world Rust desktop systems programming with comprehensive tests.
  ],
)
