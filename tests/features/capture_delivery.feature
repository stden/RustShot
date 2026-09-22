@tray_capture_window
Feature: Active-window delivery and partial failures
  Scenario Outline: Each outcome names the outputs the user received
    When window delivery fails at "<failure>"
    Then window delivery attempts "<attempts>" in order
    And window delivery reports "<message>"
    Examples:
      | failure      | attempts               | message                                                                                               |
      | nowhere      | capture,clipboard,file | window.png                                                                                            |
      | capture      | capture                | Failed to capture active window: window closed                                                        |
      | clipboard    | capture,clipboard,file | Window screenshot saved to window.png, but clipboard delivery failed: clipboard busy                   |
      | file         | capture,clipboard,file | Window screenshot copied to clipboard, but file save failed: disk full                                 |
      | both outputs | capture,clipboard,file | Window screenshot clipboard delivery failed: clipboard busy; file save also failed: disk full          |
