Feature: Recording frame-buffer helpers
  Pure helpers that convert and pack pixel data for the H.264 recording
  pipeline: RGBA-to-BGRA bottom-up conversion and u32 pair packing.

  # --- rgba_to_bgra_bottom_up ---

  Scenario: A 1x1 RGBA pixel converts to BGRA with swapped channels
    Given a 1x1 RGBA frame with pixel (10, 20, 30, 255)
    When the RGBA frame is converted to bottom-up BGRA
    Then the BGRA output pixel is (30, 20, 10, 255)

  Scenario: A 1x2 frame is flipped vertically and channels are swapped
    Given a 1x2 RGBA frame with top pixel (1, 2, 3, 4) and bottom pixel (5, 6, 7, 8)
    When the RGBA frame is converted to bottom-up BGRA
    Then the bottom-up BGRA first pixel is (7, 6, 5, 8)
    And the bottom-up BGRA second pixel is (3, 2, 1, 4)

  Scenario: Converting a frame twice restores both row order and channels
    Given a 1x2 RGBA frame with top pixel (1, 2, 3, 4) and bottom pixel (5, 6, 7, 8)
    When the RGBA frame is converted to bottom-up BGRA
    And the BGRA frame of size 1x2 is converted to bottom-up BGRA again
    Then the double-converted output matches the original RGBA frame

  Scenario: Output length equals input length
    Given a 2x2 RGBA frame with all zero pixels
    When the RGBA frame is converted to bottom-up BGRA
    Then the BGRA output has 16 bytes

  # --- pack_u32_pair ---

  Scenario: The high word occupies the upper 32 bits
    Given the u32 pair (1920, 1080)
    When the pair is packed into a u64
    Then the high word of the packed u64 is 1920
    And the low word of the packed u64 is 1080

  Scenario: Both zeros pack to zero
    Given the u32 pair (0, 0)
    When the pair is packed into a u64
    Then the packed u64 is 0

  Scenario: Maximum u32 values fill all 64 bits
    Given the u32 pair (4294967295, 4294967295)
    When the pair is packed into a u64
    Then the packed u64 is 18446744073709551615

  Scenario Outline: Round-trip extraction recovers both words
    Given the u32 pair (<hi>, <lo>)
    When the pair is packed into a u64
    Then the high word of the packed u64 is <hi>
    And the low word of the packed u64 is <lo>

    Examples:
      | hi   | lo   |
      | 30   | 1    |
      | 1920 | 1080 |
      | 0    | 42   |
