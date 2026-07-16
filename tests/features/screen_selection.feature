Feature: Screen index resolution
  A CLI capture targets one display. An explicit index is bounds-checked and
  rejected with a human-readable message that names the valid range; without an
  index the first screen is used. Zero detected screens is always an error.

  Scenario: No screens detected is a clear error
    When screen index none is resolved among 0 screens
    Then screen resolution fails with "No screens detected"

  Scenario: An out-of-range index names the valid range
    When screen index 5 is resolved among 2 screens
    Then screen resolution fails with "Screen index 5 is out of range (0-1)"

  Scenario: A valid explicit index is used as-is
    When screen index 1 is resolved among 2 screens
    Then the selected screen index is 1

  Scenario: Without an explicit index the first screen is used
    When screen index none is resolved among 3 screens
    Then the selected screen index is 0
