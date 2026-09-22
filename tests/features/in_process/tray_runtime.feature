@in_process @native_tray
Feature: Native tray availability
  Scenario: Creating the tray also enables native event delivery
    When the native tray icon and menu are created
    Then the tray is available and queued native events are dispatched
