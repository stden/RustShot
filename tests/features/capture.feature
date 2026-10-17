Feature: Pixel channel conversion and screen selection
  RustShot captures screenshots via the `screenshots` crate, which returns
  BGRA pixel data on Windows. These helpers normalize pixel formats and
  select the target monitor.

  # --- bgra_to_rgba ---

  Scenario: BGRA to RGBA swaps red and blue channels
    Given a BGRA pixel buffer with bytes (10, 20, 30, 255)
    When the buffer is converted from BGRA to RGBA
    Then the pixel buffer contains bytes (30, 20, 10, 255)

  Scenario: BGRA to RGBA applied twice is the identity
    Given a BGRA pixel buffer with bytes (10, 20, 30, 255)
    When the buffer is converted from BGRA to RGBA
    And the buffer is converted from BGRA to RGBA
    Then the pixel buffer contains bytes (10, 20, 30, 255)

  Scenario: BGRA to RGBA leaves green and alpha unchanged
    Given a BGRA pixel buffer with bytes (0, 200, 0, 128)
    When the buffer is converted from BGRA to RGBA
    Then the pixel buffer contains bytes (0, 200, 0, 128)

  Scenario: Empty BGRA buffer is a no-op
    Given an empty BGRA pixel buffer
    When the buffer is converted from BGRA to RGBA
    Then the pixel buffer is empty

  # --- rgba_to_rgb_bytes ---

  Scenario: RGBA to RGB drops the alpha channel
    Given a 1-pixel RGBA buffer with bytes (100, 150, 200, 255)
    When the RGBA buffer is converted to RGB
    Then the RGB output contains bytes (100, 150, 200)

  Scenario: RGBA to RGB output length is three-quarters of input
    Given an RGBA buffer of 4 pixels all set to (1, 2, 3, 255)
    When the RGBA buffer is converted to RGB
    Then the RGB output has 12 bytes

  Scenario: Empty RGBA buffer yields empty RGB output
    Given an empty RGBA pixel buffer
    When the RGBA buffer is converted to RGB
    Then the RGB output has 0 bytes

  # --- resolve_screen_index ---

  Scenario: No requested index always picks screen 0
    Given there are 3 screens available
    When no screen index is requested
    Then screen index 0 is resolved

  Scenario Outline: A valid explicit index is returned unchanged
    Given there are <count> screens available
    When screen index <index> is requested
    Then screen index <index> is resolved

    Examples:
      | count | index |
      | 1     | 0     |
      | 3     | 0     |
      | 3     | 2     |

  Scenario: Out-of-range index is an error
    Given there are 3 screens available
    When screen index 5 is requested
    Then screen index resolution fails

  Scenario: Zero screens always fails even with no index
    Given there are 0 screens available
    When no screen index is requested
    Then screen index resolution fails

  # --- default_screen_index ---

  Scenario: Primary screen is preferred over the first
    Given the screen primary flags are [false, true, false]
    When the default screen index is computed
    Then the default screen index is 1

  Scenario: First screen is the fallback when none is primary
    Given the screen primary flags are [false, false, false]
    When the default screen index is computed
    Then the default screen index is 0

  Scenario: Single primary screen at index 0
    Given the screen primary flags are [true]
    When the default screen index is computed
    Then the default screen index is 0

  Scenario: Empty screen list returns no index
    Given the screen primary flags are []
    When the default screen index is computed
    Then no default screen index exists
