Feature: Tray menu and icon requirements
  Tray events are translated into domain actions instead of raw strings. The
  generated camera icon is a complete 32x32 RGBA image whose opacity pattern
  is horizontally symmetric.

  Scenario Outline: Every current tray id resolves to its action
    When tray menu id "<id>" is resolved
    Then the tray action is "<action>"

    Examples:
      | id             | action        |
      | capture        | Capture       |
      | capture_window | CaptureWindow |
      | open_folder    | OpenFolder    |
      | start_record   | ToggleRecord  |
      | settings       | Settings      |
      | exit           | Exit          |

  Scenario: The legacy stop id resolves to the recording toggle
    When tray menu id "stop_record" is resolved
    Then the tray action is "ToggleRecord"

  Scenario: An unknown tray id has no action
    When tray menu id "unknown" is resolved
    Then no tray action is resolved

  @tray-16.4
  Scenario: The default tray icon satisfies its whole-buffer invariants
    When the default tray icon pixels are generated
    Then the tray icon is a 32 x 32 RGBA buffer
    And every tray icon alpha value is binary
    And the tray icon opacity is horizontally symmetric
    And every opaque tray icon pixel is camera blue or lens white
