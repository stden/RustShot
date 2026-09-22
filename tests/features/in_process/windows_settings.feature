@in_process @windows @tray_settings
Feature: Native Windows settings interactions
  Scenario: Settings can be navigated and cancelled entirely by keyboard
    Given the native Windows settings dialog is open
    Then every interactive setting is reachable in the expected Tab order
    When Escape is sent to the settings dialog
    Then the settings dialog closes and can be opened again

  Scenario: Repeated Settings requests reuse the open dialog
    Given the native Windows settings dialog is open
    When settings is requested a second time
    Then the existing settings window is reused

  Scenario: Widening settings gives folder paths more room
    Given the native Windows settings dialog is open
    When the settings window is widened
    Then the screenshot folder field expands

  @mcp_capture_window
  Scenario: A listed window can be captured by ID with an inline MCP preview
    Given the native Windows settings dialog is open
    When the visible settings window is captured through the MCP window tool
    Then the selected window is saved as a non-empty PNG

  Scenario: Valid form values preserve every setting
    When the settings form contains
      | screenshots | recordings | format | FPS | auto-start |
      | screenshots | recordings | JPG    | 60  | false      |
    Then the form preserves the folders, JPG format, 60 FPS, and disabled auto-start

  Scenario Outline: Invalid settings are rejected before saving
    When the settings form contains
      | screenshots   | recordings   | format | FPS   | auto-start |
      | <screenshots> | <recordings> | PNG    | <fps> | false      |
    Then the settings form rejects them with "<message>"
    Examples:
      | screenshots | recordings | fps | message                                  |
      |             | recordings | 30  | Screenshots folder cannot be empty.      |
      | screenshots |            | 30  | Recordings folder cannot be empty.       |
      | screenshots | recordings | 0   | Recording FPS must be a positive number. |
      | screenshots | recordings | abc | Recording FPS must be a positive number. |
      | screenshots | recordings | 241 | Recording FPS must not exceed 240.       |
