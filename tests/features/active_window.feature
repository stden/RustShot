Feature: Active window capture boundary
  Capturing the active window is Windows-only. The public API never panics:
  on Windows it saves an image (or reports a readable error when there is no
  capturable foreground window, as on a service session), elsewhere it always
  reports the platform error. Either way the caller gets a Result, never a
  crash.

  Scenario: Capturing the active window yields a file or a readable error
    When the active window is captured into a temporary directory
    Then the capture yields a saved image file or a human-readable error
