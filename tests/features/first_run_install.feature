Feature: First-run Windows startup installation
  Auto-start uses a stable per-user executable rather than whichever temporary
  or downloaded copy happened to be launched. The policy is specified through
  simulated handlers so this feature never writes to the registry or filesystem.

  Scenario: Enabled startup installs and hands off to the stable copy
    Given auto-start is enabled for a RustShot executable outside its install directory
    When startup preparation runs through simulated Windows handlers
    Then the stable startup path is under local app data at "Programs/RustShot/RustShot.exe"
    And startup preparation performs these actions in order:
      | action                      |
      | create install directory    |
      | copy executable             |
      | register installed executable |
      | launch installed executable |
    And the startup caller is told to exit

  Scenario: An existing stable installation is handed off without replacement
    Given auto-start is enabled for a RustShot executable outside its install directory
    And a stable RustShot installation already exists
    When startup preparation runs through simulated Windows handlers
    Then startup preparation performs these actions in order:
      | action                        |
      | register installed executable |
      | launch installed executable   |
    And the startup caller is told to exit

  Scenario: Re-enabling auto-start in Settings does not interrupt a portable session
    Given a portable RustShot session outside its install directory re-enables auto-start
    When Settings startup synchronization runs through simulated Windows handlers
    Then the stable startup path is under local app data at "Programs/RustShot/RustShot.exe"
    And startup preparation performs these actions in order:
      | action                        |
      | create install directory      |
      | copy executable               |
      | register installed executable |
    And the current RustShot session remains running
