Feature: CLI helper functions
  Pure helper functions for CLI argument inspection, format resolution,
  screen list formatting, and selection error classification.

  # --- cli_format_flag_present_in ---

  Scenario: No format flag returns false
    Given the CLI args are ["rustshot", "--output", "shot.png"]
    When the CLI args are checked for a format flag
    Then no format flag is present in the args

  Scenario: Long --format flag is detected
    Given the CLI args are ["rustshot", "--format", "jpg"]
    When the CLI args are checked for a format flag
    Then a format flag is present in the args

  Scenario: Short attached -f flag is detected
    Given the CLI args are ["rustshot", "-fpng"]
    When the CLI args are checked for a format flag
    Then a format flag is present in the args

  Scenario: --format consumed as --output value is not a format flag
    Given the CLI args are ["rustshot", "--output", "--format"]
    When the CLI args are checked for a format flag
    Then no format flag is present in the args

  Scenario: Only the program name produces no format flag
    Given the CLI args are ["rustshot"]
    When the CLI args are checked for a format flag
    Then no format flag is present in the args

  # --- format_screen_list_output ---

  Scenario: Single screen entry appears in the output
    Given the screen list contains ["Screen 0: 1920x1080"]
    When the screen list is formatted for output
    Then the screen list output contains "Screen 0: 1920x1080"

  Scenario: Multiple screens each appear in the output
    Given the screen list contains ["Screen 0: 1920x1080", "Screen 1: 2560x1440"]
    When the screen list is formatted for output
    Then the screen list output contains "Screen 0: 1920x1080"
    And the screen list output contains "Screen 1: 2560x1440"

  Scenario: Output always contains the header line
    Given the screen list contains []
    When the screen list is formatted for output
    Then the screen list output contains "Available screens:"

  # --- selection_stderr_message ---

  Scenario: A cancelled message returns no error string
    When the selection stderr message "Selection cancelled" is classified
    Then the selection error result is absent

  Scenario: An actual error line is passed through
    When the selection stderr message "error: failed to open display" is classified
    Then the selection error result is present

  Scenario: A blank message is passed through as a present result
    When the selection stderr message "" is classified
    Then the selection error result is present
