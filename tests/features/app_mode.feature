Feature: Application mode routing
  RustShot chooses how to run from its CLI flags. The decision follows a fixed
  priority: select-record beats select, select beats any plain CLI indicator,
  and with no flags at all the app launches into the system tray.

  Scenario: No flags launches the tray
    Given no CLI flags are set
    When the application mode is decided
    Then the application mode is "Tray"

  Scenario Outline: A single flag selects its mode
    Given the "<flag>" flag is set
    When the application mode is decided
    Then the application mode is "<mode>"

    Examples:
      | flag            | mode         |
      | select_record   | SelectRecord |
      | select          | Selection    |
      | cli             | Cli          |
      | list            | Cli          |
      | window          | Cli          |
      | has_output      | Cli          |
      | has_screen      | Cli          |
      | format_explicit | Cli          |

  Scenario: Select-record wins over every other flag
    Given the "select_record" flag is set
    And the "select" flag is set
    And the "cli" flag is set
    When the application mode is decided
    Then the application mode is "SelectRecord"

  Scenario: Select beats a plain CLI indicator
    Given the "select" flag is set
    And the "list" flag is set
    When the application mode is decided
    Then the application mode is "Selection"
