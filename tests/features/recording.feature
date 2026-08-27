Feature: Recording validation and frame timing
  Before a recording starts, the requested FPS is validated. Frame timing is
  then expressed in 100-nanosecond units (the unit Windows Media Foundation
  timestamps use), and capture dimensions are rounded to even numbers as H.264
  requires - upward for every value the checked helper accepts, which is every
  value except u32::MAX.

  Scenario: Zero FPS is rejected
    When recording start is validated with 0 FPS
    Then validation fails with "Recording FPS must be greater than 0"

  Scenario: FPS above the maximum is rejected
    When recording start is validated with 300 FPS
    Then validation fails with "Recording FPS must be 240 or less"

  Scenario: A valid FPS is accepted
    When recording start is validated with 30 FPS
    Then validation succeeds

  Scenario Outline: Frame duration in 100-nanosecond units
    When the frame duration for <fps> FPS is computed
    Then the frame duration is <hns> hundred-nanosecond units

    Examples:
      | fps | hns    |
      | 30  | 333333 |
      | 60  | 166666 |
      | 24  | 416666 |
      | 0   | 0      |

  Scenario Outline: Dimensions are rounded to the nearest even value
    When <value> is rounded to even
    Then the rounded value is <result>

    # The last row is the one input the unchecked helper maps DOWNWARD: u32::MAX
    # is odd and 2^32 is unrepresentable, so the saturating add returns the input
    # and clearing bit 0 lands one below it. Spelled out here because this file is
    # the executable specification, and stating "rounded up" without the exception
    # is what let the rustdoc promise a contract the code never honoured.
    Examples:
      | value      | result     |
      | 1920       | 1920       |
      | 1921       | 1922       |
      | 1          | 2          |
      | 0          | 0          |
      | 4294967295 | 4294967294 |

  Scenario: Checked rounding still rounds ordinary dimensions up
    When the dimension 1921 is rounded to even with overflow checking
    Then the checked rounding yields 1922

  Scenario: A dimension at the u32 limit cannot be rounded and is rejected
    When the dimension 4294967295 is rounded to even with overflow checking
    Then the checked rounding fails with "Recording dimension is too large to round to an even value"

  Scenario: Converting a frame into an exact-size BGRA buffer succeeds
    When a 1x2 RGBA frame is converted into a 8-byte BGRA buffer
    Then the conversion succeeds

  Scenario: Converting a frame into an undersized BGRA buffer is rejected
    When a 1x2 RGBA frame is converted into a 4-byte BGRA buffer
    Then the conversion fails mentioning "BGRA output buffer is too short"

  Scenario: A zero-sized region cannot start a recording
    When a recording start is requested for a zero-sized region
    Then the request is rejected before any recorder is created
