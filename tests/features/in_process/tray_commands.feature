@in_process
Feature: Tray commands, hotkeys, and orderly shutdown
  @tray_exit
  Scenario Outline: Exit preserves recording finalization and reports failures
    When tray exit is requested with recording "<recording>" and "<failure>"
    Then tray operations are "<operations>"
    And tray exit reports "<result>"
    Examples:
      | recording | failure           | operations                     | result                    |
      | active    | no failure        | stop recording,close selection | ready                     |
      | absent    | no failure        | close selection                | ready                     |
      | active    | stop failure      | stop recording                 | stop marker denied        |
      | absent    | selection failure | close selection                | selection could not close |

  @tray_open_folder
  Scenario Outline: Open Folder prepares the configured directory first
    When the screenshots folder is opened after directory creation "<creation>"
    Then tray operations are "<operations>"
    Examples:
      | creation | operations                 |
      | succeeds | prepare folder,open folder |
      | fails    | prepare folder             |

  Scenario Outline: Hotkey presses invoke the same commands as the tray menu
    When registered shortcut "<shortcut>" is "<state>"
    Then the shortcut dispatches "<action>"
    Examples:
      | shortcut       | state    | action         |
      | PrintScreen    | pressed  | capture        |
      | Alt+PrintScreen| pressed  | capture_window |
      | Ctrl+Shift+R   | pressed  | start_record   |
      | PrintScreen    | released | nothing        |
      | Alt+PrintScreen| released | nothing        |
      | Ctrl+Shift+R   | released | nothing        |

  Scenario: A conflicting shortcut prevents a partial registration
    When Alt+PrintScreen registration fails because it is already in use
    Then the registration error identifies the conflicting shortcut
