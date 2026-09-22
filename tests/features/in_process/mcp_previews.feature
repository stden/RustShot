@in_process @mcp_capture_screen @mcp_capture_window
Feature: MCP saved-image previews
  Scenario Outline: Screenshot replies contain the saved path and a decodable image
    When a saved "<format>" screenshot is prepared for an MCP reply
    Then the MCP preview has MIME type "<mime>" and exactly the saved bytes
    Examples:
      | format | mime       |
      | png    | image/png  |
      | jpg    | image/jpeg |
      | bmp    | image/bmp  |

  Scenario: Losing an inline preview does not conceal a successful file save
    When a screenshot file disappears before its MCP preview is read
    Then MCP still reports the saved path and explains why the preview is unavailable
