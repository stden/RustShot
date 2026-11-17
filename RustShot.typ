// Illustrated learning material. Rebuild with: rtk proxy mise run docs:pdf
#let navy = rgb("#14223d")
#let blue = rgb("#356dff")
#let teal = rgb("#168263")
#let cyan = rgb("#22b8cf")
#let orange = rgb("#bd651c")
#let purple = rgb("#9850a4")
#let muted = rgb("#52627b")
#let pale = rgb("#f4f7fc")

#set document(
  title: "RustShot — An Illustrated Field Guide",
  author: "RustShot Contributors",
  description: "A practical textbook on screenshot capture, Windows recording, Rust architecture, testing, and performance.",
  keywords: ("RustShot", "Rust", "screenshots", "recording", "testing", "performance"),
)
#set page(
  paper: "a4",
  margin: (x: 18mm, top: 17mm, bottom: 18mm),
  header: context {
    if counter(page).get().first() > 1 {
      set text(font: ("Segoe UI", "Libertinus Serif"), size: 8pt, fill: muted)
      grid(columns: (1fr, 1fr), [RUSTSHOT / ILLUSTRATED FIELD GUIDE], align(right)[LEARN · BUILD · VERIFY])
      v(3pt)
      line(length: 100%, stroke: 0.5pt + rgb("#dce3ef"))
    }
  },
  footer: context {
    if counter(page).get().first() > 1 {
      set text(font: ("Segoe UI", "Libertinus Serif"), size: 8pt, fill: muted)
      grid(columns: (1fr, auto), [Source-driven explanations · editable vector illustrations], text(fill: blue, weight: "bold")[#counter(page).display()])
    }
  },
)
#set text(font: "Libertinus Serif", size: 11.8pt, fill: navy, lang: "en")
#set par(leading: 0.62em, spacing: 0.65em)
#set heading(numbering: none)
#show heading.where(level: 1): it => block(above: 5pt, below: 9pt)[
  #text(font: ("Segoe UI", "Libertinus Serif"), size: 26pt, weight: "bold", fill: navy)[#it.body]
]
#show heading.where(level: 2): it => block(above: 11pt, below: 5pt)[
  #text(font: ("Segoe UI", "Libertinus Serif"), size: 13pt, weight: "bold", fill: blue)[#it.body]
]
#show raw: set text(font: ("Cascadia Mono", "DejaVu Sans Mono", "Libertinus Serif"), size: 9pt)
#show raw.where(block: true): it => block(
  width: 100%, inset: 10pt, fill: pale, radius: 6pt,
  stroke: 0.5pt + rgb("#dde6f4"),
)[#it]
#set table(inset: 6pt, stroke: 0.5pt + rgb("#dce3ef"))
#show table.cell: set text(size: 10.5pt)
#set list(indent: 12pt, body-indent: 5pt, spacing: 4pt)
#set enum(indent: 14pt, body-indent: 5pt, spacing: 4pt)

#let chapter(number, title, goal) = {
  pagebreak()
  text(font: ("Segoe UI", "Libertinus Serif"), size: 9pt, weight: "bold", fill: blue)[CHAPTER #number]
  heading(level: 1, title)
  text(size: 11.5pt, fill: muted)[#goal]
  v(7pt)
}
#let card(title, body, accent: blue) = block(
  width: 100%, inset: 10pt, radius: 7pt,
  fill: accent.lighten(93%), stroke: 0.6pt + accent.lighten(60%),
)[
  #text(font: ("Segoe UI", "Libertinus Serif"), size: 10pt, weight: "bold", fill: accent)[#title]
  #v(3pt)
  #body
]
#let pair(left, right) = grid(columns: (1fr, 1fr), gutter: 10pt, left, right)
#let illustration(path, caption, width: 100%) = block(above: 8pt, below: 8pt)[
  #align(center, image(path, width: width))
  #v(3pt)
  #text(size: 9.2pt, fill: muted)[#caption]
]
#let source(body) = text(size: 8.5pt, fill: muted)[Read the implementation: #body]
#let exercise(body) = card("TRY IT", body, accent: teal)

// Cover
#v(9mm)
#text(font: ("Segoe UI", "Libertinus Serif"), size: 11pt, weight: "bold", fill: blue)[A PRACTICAL TEXTBOOK FOR CURIOUS BUILDERS]
#v(7mm)
#text(font: ("Segoe UI", "Libertinus Serif"), size: 58pt, weight: "bold", fill: navy)[RustShot]
#v(2mm)
#text(font: ("Segoe UI", "Libertinus Serif"), size: 25pt, fill: blue)[An Illustrated Field Guide]
#v(5mm)
#text(size: 15pt, fill: muted)[Capture pixels. Trace events. Measure performance.]
#v(9mm)
#image("docs/diagrams/textbook-cover.svg", width: 100%)
#v(7mm)
#pair(
  card("FROM FIRST CAPTURE", [Use the tray and CLI. Understand what is copied, saved, and recorded.], accent: teal),
  card("TO VERIFIED CHANGES", [Read the Rust architecture. Test failure paths. Measure the work that matters.], accent: purple),
)
#v(6mm)
Designed for readers who know basic Rust and want to understand a real desktop
application. Each chapter connects an observable action to the code responsible
for it, then gives you a small experiment.
#v(4mm)
#text(size: 9pt, fill: muted)[Illustrations are editable vectors. The cover is a synthetic desktop scene, not a product screenshot.]

#chapter("00", "Your learning route", "Start with visible behavior, then follow the data into the implementation.")

RustShot combines several kinds of work: global hotkeys, desktop windows, pixel
buffers, clipboard ownership, file encoding, and Windows video recording. Learn
one boundary at a time. You do not need to read the largest source file first.

#pair(
  card("1 / OBSERVE", [Make a region capture and a foreground-window capture. Explain why their destinations differ.], accent: blue),
  card("2 / BUILD", [Use the managed toolchain. Distinguish a portable release from a build for your own CPU.], accent: teal),
)
#v(8pt)
#pair(
  card("3 / TRACE", [Follow the tray into a selection child, a clipboard operation, or a recording worker.], accent: purple),
  card("4 / MEASURE", [Write a regression test, isolate a hot path, and compare the same workload before and after a change.], accent: orange),
)

== Four modes, one executable

#table(
  columns: (auto, 1fr, 1fr),
  table.header([*Mode*], [*Entry*], [*Responsibility*]),
  [Tray], [No capture flags], [Keep the menu and hotkeys available],
  [CLI], [For example, `--cli` or `--output`], [Capture, save, print the path, exit],
  [Selection], [Internal `--select`], [Show an overlay and deliver a region image],
  [Select-record], [Internal `--select-record`], [Select a region and run recording],
)

The mode decision is pure policy. It chooses *select-record first*, then
selection, then CLI, with tray as the fallback. Window creation and capture
happen only after that decision.

#exercise[
Find `AppModeRequest` and `decide_app_mode` in `src/cli.rs`. Predict the
result when both selection flags are true. Then locate the test that protects
that priority.
]
#source[`src/cli.rs`, `src/main.rs`]

#chapter("01", "Take your first capture", "Learn the output contract before inspecting the implementation.")

Start RustShot without capture flags to enter tray mode. The icon provides the
menu, while global hotkeys trigger the most common actions.

#table(
  columns: (auto, 1fr),
  table.header([*Action*], [*What you receive*]),
  [PrintScreen], [Drag a region; its image is copied to the clipboard.],
  [Alt+PrintScreen], [On Windows, copy the foreground window and save a copy in the background.],
  [Ctrl+Shift+R], [On Windows, select a recording region and start recording.],
  [Ctrl+Shift+R again], [Stop the recording and finalize the MP4 file.],
)

#illustration("docs/diagrams/clipboard-pipeline.svg", [The window hotkey has a clipboard destination and a separate file-save step.])

== Clipboard and file output

A region capture is intended for immediate sharing: paste it into another
application with Ctrl+V. It does not automatically create a screenshot file.
The window hotkey also keeps a saved copy in the configured screenshot directory.
CLI captures produce files.

The default screenshot directory is `Pictures/RustShot`; recordings go into
its `recordings` subdirectory. Both locations can be changed in settings.

#exercise[
Capture a region and paste it. Next, capture a foreground window and paste
again. Use the tray menu to open the screenshot directory and compare the
results. Do not use private material for a shared demonstration.
]
#source[`RustShotApp::handle_capture`, `RustShotApp::handle_window_capture` in `src/main.rs`]

#chapter("02", "A repeatable build with mise", "Use the repository's tasks to make the build and its checks explicit.")

The project configuration selects Rust, RTK, Python, and Typst. Inspect
`mise.toml` when opening a checkout, then trust the configuration and install
the configured tools.

```sh
rtk proxy mise trust
rtk proxy mise install
rtk proxy mise run ci
rtk proxy mise run build
```

#pair(
  card("PORTABLE RELEASE", [`mise run build` uses optimization level 3, full LTO, and one code-generation unit. Use this configuration when distributing binaries.], accent: blue),
  card("THIS MACHINE'S CPU", [`mise run build:native` also sets `target-cpu=native`. Use the resulting binary on a CPU with compatible features.], accent: orange),
)

== Choose the task for the question

#table(
  columns: (auto, 1fr),
  table.header([*Task*], [*What it checks or creates*]),
  [`check`], [Compilation of all Cargo targets],
  [`test`], [Application and duplication detector tests],
  [`ci`], [Formatting, Clippy, duplicate-code checks, and tests],
  [`coverage`], [LLVM coverage exported to `target/coverage.json`],
  [`bench`], [Criterion benchmark results],
  [`docs:pdf`], [This textbook as `RustShot.pdf`],
)

Tasks invoke the existing Rust tools rather than replacing their behavior.
Use `rtk proxy mise exec -- <command>` for a managed tool without a dedicated
task. RTK reduces command output; `rtk recall` retrieves omitted detail.

#exercise[
Run `rtk proxy mise tasks ls`. Find the task that checks formatting without
changing files, then rebuild this textbook with `rtk proxy mise run docs:pdf`.
]
#source[`mise.toml`, `rust-toolchain.toml`, `Cargo.toml`]

#chapter("03", "Follow the process boundary", "Separate the tray's lifetime from the work performed by its children.")

The tray remains available while a child process owns the selection overlay.
The same executable is launched with different internal flags, so there is no
second application to distribute.

#illustration("docs/diagrams/process-isolation.svg", [Process boundaries and their communication paths. Image transport differs by platform.])

On Linux, a selection child sends width, height, and RGBA bytes through stdout.
The tray reads that image and retains the clipboard owner after the child exits.
On Windows and macOS, the selection child copies directly to the clipboard.

Recording also stays in a child process. Its capture worker runs there, not
in the tray. The parent holds a pipe to the child's standard input; closing
the pipe delivers EOF and requests a graceful stop.

#exercise[
Draw a box around each process in the diagram. Put the recording worker inside
the correct box, then explain which resources must remain alive after a
selection child exits on Linux.
]
#source[`src/child_process.rs`, `src/selection_clipboard.rs`, `src/main.rs`]

#chapter("04", "Copy first, save afterward", "A responsive hotkey depends on the order of operations.")

Alt+PrintScreen should capture the window that was in the foreground when the
hotkey was handled. Capturing later in a background worker would leave a gap in
which focus could change. RustShot captures and copies on the hotkey thread,
then moves the captured image into a save worker.

#illustration("docs/diagrams/clipboard-pipeline.svg", [The same captured pixels are copied before they are handed to file encoding.])

The coordinator accepts capture, copy, and save callbacks. That makes the
ordering testable without opening a window or changing a real clipboard.
The clipboard helper retains an `arboard::Clipboard` instance in the tray.

#table(
  columns: (auto, 1fr),
  table.header([*Failure*], [*Expected result*]),
  [Capture fails], [Log the error; do not copy or save an image.],
  [Clipboard is unavailable], [Log the error; still try to save the captured image.],
  [Saving fails], [Log the error; keep the successful clipboard copy available.],
)

#card("TEST THE PROMISE", [
Use a recognizable two-pixel image. In the copy callback, record the caller's
thread and mark the image as copied. In the save callback, assert that copying
has already happened and that the pixels are unchanged.
], accent: purple)

#exercise[
Find `active_window_capture_copies_pixels_before_background_save`. Explain
why checking only that a worker was created would miss the original bug.
]
#source[`handle_window_capture_with` in `src/main.rs`, `src/clipboard.rs`]

#chapter("05", "Make the crosshair reliable", "Treat geometry, drawing, and presentation as separate responsibilities.")

The overlay begins with a captured background. When the cursor moves, it
restores the previous crosshair or border and draws the new primitives.
`OverlayDamage` records the geometry needed to restore those pixels.

#illustration("docs/diagrams/incremental-rendering.svg", [Incremental CPU drawing and presentation are different stages. A smaller drawing update does not imply a smaller Windows blit.])

== Why Windows presents a complete frame

Presenting the erase and draw rectangles separately can expose an intermediate
state. Windows uses one complete presentation and calls `GdiFlush` before the
next update can overwrite the DIB pixels. Other platforms can present the
changed rectangles.

== Geometry must agree with the buffer

Each overlay follows its monitor's physical bounds. During a DPI transition,
resize events may be stale, so drawing uses the allocated buffer's width and
height as its stride and limits. Border coordinates clamp to the last valid
pixel rather than assuming an event's size is already current.

On Windows, cursor input is coalesced until a display deadline. The scheduler
uses the highest reported monitor refresh rate, with a fallback when unavailable.
A pending final position still receives a frame after the mouse stops.

#exercise[
List three independent regressions to check: crosshair visibility during rapid
movement, coverage of all screen edges, and the final position after input stops.
Explain why a fast pixel benchmark alone cannot prove these behaviors.
]
#source[`src/selection.rs`, `tools/benchmark_overlay_windows.ps1`]

#chapter("06", "Read pixels like a data format", "Channel order, row order, and alpha each need an explicit contract.")

An image is a byte buffer plus metadata: width, height, pixel format, and row
layout. A correct buffer length does not guarantee correct colors or orientation.
Name the source and destination formats before writing a conversion.

#illustration("docs/diagrams/pixel-layout.svg", [Swapping channels and reversing rows solve different problems.])

== Fuse compatible passes

Windows GDI capture produces BGRA bytes and can leave alpha unset.
`bgra_to_opaque_rgba` swaps red and blue and sets alpha to 255 in one pass.
That avoids a second full traversal of the same image.

```rust
// The operation on one pixel; the real helper iterates the image.
let mut pixel = [10u8, 20, 30, 0]; // BGRA
pixel.swap(0, 2);
pixel[3] = 255;
assert_eq!(pixel, [30, 20, 10, 255]); // opaque RGBA
```

== Reuse storage in a frame loop

Allocating a destination for every frame adds work beyond the conversion.
The `_into` helpers write into a caller-owned buffer. When the source is already
BGRA, the recording path can reverse rows without swapping red and blue again.
Padding for codec dimensions is another operation to measure separately.

#exercise[
A two-row image has red pixels in its top row and blue pixels in its bottom
row. Describe the result of reversing rows without swapping channels. Then
describe the result of swapping channels without reversing rows.
]
#source[`src/capture.rs`, `src/record.rs`, `benches/benchmarks.rs`]

#chapter("07", "Recording is a lifecycle", "Starting a worker is only the beginning; a playable MP4 needs finalization.")

Windows recording uses Media Foundation to encode H.264 video in an MP4
container. The selection child chooses a region, starts a `RecordingSession`,
and waits for a stop request or worker completion.

#pair(
  card("1 / PREPARE", [Validate FPS and region dimensions. Initialize COM, Media Foundation, and the sink writer.], accent: blue),
  card("2 / CAPTURE", [Capture the selected region and prepare bottom-up BGRA pixels in reused storage.], accent: teal),
)
#v(9pt)
#pair(
  card("3 / ENCODE", [Submit timestamped samples to the sink writer. Keep capture and format conversion distinct in measurements.], accent: purple),
  card("4 / FINALIZE", [Signal the worker, join it, finalize the MP4, and release platform resources.], accent: orange),
)

== The stop signal crosses a process boundary

The tray owns the recording child's stdin pipe. A second recording hotkey closes
that pipe. EOF reaches the child's stop listener, which lets the session finish
its shutdown sequence. The parent can request a stop without transferring a
Rust channel across processes.

== Dimensions belong to the codec contract

The encoder needs even dimensions and enforces a minimum encoded size.
The encoded frame can therefore include padding around the selected region.
Test source dimensions and destination dimensions independently, including odd
widths and heights.

#card("KEEP THE STEPS OBSERVABLE", [
A capture error, a conversion error, and a writer error may all stop a recording.
Keep their messages specific so a report can identify the failing stage.
Shutdown must still release GDI, COM, and Media Foundation resources.
], accent: purple)

#exercise[
Follow the stop path from `handle_recording_toggle` to `RecordingSession::stop`.
Identify the process that finalizes the MP4. Explain why terminating the child
immediately would skip part of the intended lifecycle.
]
#source[`src/child_process.rs`, `run_select_record_with`, `src/record.rs`]

#chapter("08", "Make command-line policy predictable", "A small pure decision can prevent a confusing user-facing result.")

CLI captures save files and print the resulting path. A window request uses the
foreground-window capture path on Windows. On Linux Wayland, full-desktop CLI
capture uses the screenshot portal; single-screen and window requests are
rejected because that portal path cannot honor them.

#illustration("docs/diagrams/wayland-portal.svg", [A successful portal response provides a local image. Saving determines whether its separate portal copy is removed or retained.])

```sh
rtk proxy mise exec -- target/release/RustShot.exe --list
rtk proxy mise exec -- target/release/RustShot.exe --cli --output shot.png
rtk proxy mise exec -- target/release/RustShot.exe --window --output window.jpg
```

The executable names above are for Windows. On Linux and macOS, use
`target/release/RustShot`.

== Format precedence

#table(
  columns: (auto, 1fr, 1fr),
  table.header([*Priority*], [*Source*], [*Example*]),
  [First], [Explicit `--format`], [`--format png` selects PNG],
  [Then], [Recognized output extension], [`--output photo.jpg` selects JPEG],
  [Fallback], [Saved default format], [Used when neither overrides it],
)

The code tracks whether the format flag was explicitly provided. Merely seeing
the parser's default value is insufficient: a default PNG value must not
silently override a requested `.jpg` filename.

== Settings are a boundary too

`settings.rs` reads and writes screenshot and recording directories, the default
format, recording FPS, and Windows autostart preference. Missing or malformed
values have defined defaults. Test escaping and round trips, especially for
Windows paths containing backslashes or non-ASCII characters.

#exercise[
Predict the selected format for `--output shot.jpg --format png` and for
`--output shot.jpg` alone. Locate `effective_cli_format` and verify both cases.
]
#source[`src/cli.rs`, `src/settings.rs`, `src/portal.rs`]

#chapter("09", "Test the boundary, not the desktop", "Use deterministic inputs to protect real user promises.")

The repository combines focused unit tests, integration tests, property tests,
and executable Gherkin scenarios. They answer different questions: a named
regression protects one failure, while a property test explores an invariant
over many generated inputs.

#illustration("docs/diagrams/test-pyramid.svg", [Use the most focused test that can verify the behavior. Desktop checks complement pure logic tests.], width: 80%)

== Inject effects into a coordinator

A helper can accept callbacks for capture and saving, while pure functions decide
mode, format, geometry, or error messages. Tests supply tiny images and temporary
paths. Production supplies the platform operations.

```rust
use rustshot::{capture::Format, cli};
use std::path::Path;

let format = cli::effective_cli_format(
    Format::Png, false, Some(Path::new("shot.jpg")), Format::Bmp,
);
assert_eq!(format, Format::Jpg);
```

For a behavior change, first add a test that fails. Make the minimal fix, rerun
the focused test, and run the complete suite through `rtk proxy mise run test`.
Use `mise run coverage` to find untested branches; inspect what the metric
includes before comparing percentages.

#exercise[
Design a test for clipboard failure that proves saving still occurs. Record a
save callback invocation; a successful return value by itself does not prove
that the callback ran.
]
#source[`src/main.rs` tests, `tests/features/`, `tests/bench_functions.rs`]

#chapter("10", "Measure the work that matters", "Separate algorithm cost from desktop behavior and system noise.")

Criterion benchmarks exercise repeatable workloads. The desktop benchmark
exercises real input and presentation. Use both: a faster conversion cannot
prove that the crosshair remains visible.

#table(
  columns: (auto, 1fr),
  table.header([*Benchmark group*], [*Question it answers*]),
  [`bgra_to_rgba/opaque_4k_*`], [Does fusing alpha and channel conversion reduce traversal work?],
  [`recording_frame_preparation`], [What changes when the output buffer is reused, rows are flipped, or dimensions need padding?],
  [`jpeg_color_preparation`], [How much does RGBA-to-RGB preparation cost before encoding?],
  [`overlay_workloads`], [How do full-frame, moving, and stationary CPU drawing differ?],
  [`overlay_redraw_scheduling`], [How much work does scheduling do for a stream of input events?],
)

```sh
rtk proxy mise exec -- cargo bench --bench benchmarks -- recording_frame_preparation
rtk proxy mise exec -- cargo bench --bench benchmarks -- jpeg_color_preparation
```

== Compare equivalent experiments

Record the revision, compiler flags, dimensions, sample settings, and power
conditions. Keep profiling and competing builds out of timed comparisons.
Criterion reports go under `target/criterion/`; throughput in these conversion
groups counts input bytes, not all memory traffic.

For the Windows overlay, build first, then run:

```sh
rtk proxy mise exec -- pwsh -NoProfile -File tools/benchmark_overlay_windows.ps1
```

Add `-CsvPath target/overlay.csv` to export the results.
The script temporarily moves and restores the cursor. Compare actual
`InputEvents`, elapsed time, resolution, and CPU milliseconds. A zero CPU
sample can reflect timer granularity. Visibility sampling checks the displayed
crosshair; it does not measure exact input-to-display latency.

#exercise[
Choose a suspected bottleneck. Name the benchmark that isolates it and one
observable desktop behavior that must remain correct after an optimization.
]
#source[`benches/benchmarks.rs`, `tools/benchmark_overlay_windows.ps1`]

#chapter("11", "A capstone: investigate a bug", "Turn a short report into a reproducible experiment and a reviewable change.")

Imagine this report: “The crosshair disappears when I move the mouse quickly,
and its lines stop before the edge of my second monitor.” There are several
possible causes. Keep the observations separate before changing the renderer.

#table(
  columns: (auto, 1fr),
  table.header([*Stage*], [*Evidence to collect*]),
  [Reproduce], [Monitor bounds, scaling, refresh rates, cursor behavior, and the exact executable used],
  [Localize], [Whether the fault is in geometry, framebuffer updates, or presentation],
  [Test], [A failing geometry or scheduling regression with deterministic inputs],
  [Fix], [The smallest change that addresses the demonstrated cause],
  [Measure], [Equivalent input rates and dimensions before and after the change],
  [Verify], [Edge coverage, visible samples, and the final pending cursor position],
)

== A useful review matrix

#pair(
  card("BEHAVIOR", [Does the crosshair reach every edge? Does a monitor crossing clear the old crosshair? Does the final frame arrive when input stops?], accent: teal),
  card("COST", [Did CPU drawing, presentation frequency, or allocation change? Was the benchmark run with the same input workload and compiler settings?], accent: orange),
)

A performance result and a correctness result belong together. A reduced CPU
measurement from fewer delivered input events is not evidence of a better
algorithm. Conversely, unchanged pixel output does not prove that the desktop
presentation is free from transient flicker.

#exercise[
Write a short change description with four parts: the user's trigger, the
observed fault, the resulting behavior, and the checks that support the claim.
Leave out claims your measurements cannot establish.
]

#card("FINAL PRACTICAL CHECK", [
Run `rtk proxy mise run ci`, build the appropriate release, and check the
affected desktop workflow. For documentation changes, rebuild this PDF and
inspect the rendered pages after updating diagrams.
], accent: blue)

#chapter("12", "Review, explain, continue", "Use these checkpoints to test whether the architecture now feels concrete.")

== Check your explanations

#table(
  columns: (auto, 1fr),
  table.header([*Question*], [*Expected explanation*]),
  [Does PrintScreen save a file?], [The region flow copies to the clipboard; it does not automatically save a file.],
  [What wins when both internal selection flags are set?], [Select-record has priority over selection.],
  [Why copy before background saving?], [Clipboard delivery should not wait for file encoding, and capture should use the foreground window at handling time.],
  [Who owns a Linux region clipboard image?], [The tray retains the clipboard owner after receiving the child image.],
  [What stops recording?], [The parent closes the child's stdin pipe; EOF requests a stop, then the worker finalizes MP4.],
  [Why are rows and channels separate?], [Reversing orientation does not change color order; channel swapping does not reverse rows.],
  [What does a redraw benchmark omit?], [OS presentation, real input delivery, and input-to-display latency.],
)

== Where to read next

#table(
  columns: (auto, 1fr),
  [`README.md`], [Usage, platform support, managed build commands],
  [`docs/capture-guide.md`], [Capture destinations, platform limits, and troubleshooting],
  [`docs/architecture-tutorial.md`], [Module map, data flows, and Rust design patterns],
  [`AGENTS.md`], [Repository rules, verification, and documentation workflow],
  [`src/cli.rs`], [Pure routing and format policy],
  [`src/selection.rs`], [Physical geometry, overlay drawing, and scheduling],
  [`src/record.rs`], [Pixel preparation and Windows recording lifecycle],
  [`benches/benchmarks.rs`], [Repeatable performance experiments],
)

#card("KEEP THE BOOK CLOSE TO THE CODE", [
The Typst source is `RustShot.typ`. Illustrations live in `docs/diagrams/`.
Build with `rtk proxy mise run docs:pdf`. Update explanations when the behavior
changes; generate test counts and performance measurements from the tools
rather than treating old results as permanent facts.
], accent: teal)

#v(6pt)
#text(size: 9pt, fill: muted)[Project: #link("https://github.com/stden/RustShot")[github.com/stden/RustShot] · License: MIT OR Apache-2.0]
