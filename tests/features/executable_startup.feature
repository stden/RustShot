Feature: Executable startup diagnostics
  RustShot is a GUI-subsystem application on Windows, so a bare launch enters
  tray mode without opening a console window. Diagnostic command-line options
  must still prove that the executable can start and report useful output
  without creating the tray or claiming global hotkeys.

  Scenario: Help proves the executable can start
    When the RustShot executable is launched with "--help"
    Then the RustShot executable exits successfully
    And the RustShot process output contains "Usage: RustShot"
    And the RustShot process output contains "Run in CLI mode (single capture, no tray)"

  Scenario: Version can be queried without entering tray mode
    When the RustShot executable is launched with "--version"
    Then the RustShot executable exits successfully
    And the RustShot executable reports its package version

  Scenario: Invalid arguments produce a visible diagnostic
    When the RustShot executable is launched with "--not-a-real-flag"
    Then the RustShot executable reports a command-line error
    And the RustShot process output contains "unexpected argument '--not-a-real-flag'"
