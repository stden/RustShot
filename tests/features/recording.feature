Feature: Recording validation and frame timing
  Before a recording starts, the requested FPS is validated. Frame timing is
  then expressed in 100-nanosecond units (the unit Windows Media Foundation
  timestamps use), and capture dimensions are rounded up to even numbers as
  H.264 requires.

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

  Scenario Outline: Dimensions are rounded up to the nearest even value
    When <value> is rounded to even
    Then the rounded value is <result>

    Examples:
      | value | result |
      | 1920  | 1920   |
      | 1921  | 1922   |
      | 1     | 2      |
      | 0     | 0      |

  Scenario Outline: Encoded dimensions are rounded to even and clamped to the H.264 minimum
    When <value> is converted to an encoded dimension
    Then the encoded dimension is <result>

    Examples:
      | value | result |
      | 0     | 0      |
      | 1     | 64     |
      | 63    | 64     |
      | 65    | 66     |
      | 1920  | 1920   |

  Scenario: Odd-dimension RGBA region is padded with opaque black in bottom-up BGRA
    When a 1 x 1 RGBA pixel (10, 20, 30, 255) is converted into a 2 x 2 bottom-up BGRA frame
    Then the bottom-up BGRA top-left pixel is (30, 20, 10, 255) and padding pixel is (0, 0, 0, 255)
