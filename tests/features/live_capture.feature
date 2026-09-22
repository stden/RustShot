Feature: Cross-platform live screen capture
  RustShot's command-line boundary must enumerate a real display, capture
  screen 0, encode a PNG, write it to the requested path, and exit promptly.
  Linux CI supplies an Xvfb display; macOS CI uses the runner's desktop.

  Scenario Outline: The CLI captures screen 0 and reports end-to-end timing
    Given the live capture scenario targets "<platform>"
    When screen 0 is captured as PNG by the RustShot CLI with timing
    Then the captured PNG is a valid non-empty screen image
    And the timed capture completes within 10000 milliseconds

    Examples:
      | platform |
      | linux    |
      | macos    |
