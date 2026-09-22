Feature: Validated recording domain types
  Recording values cross into the encoder only after construction as domain
  types. FPS has a strict range, Width and Height produce non-zero even H.264
  dimensions, frame timestamps are exact, and bitrate remains bounded.

  Scenario Outline: Strict FPS construction accepts the supported range
    When an FPS value is strictly constructed from <input>
    Then the strict FPS value is <input>

    Examples:
      | input |
      | 1     |
      | 30    |
      | 240   |

  Scenario Outline: Strict FPS construction rejects invalid values
    When an FPS value is strictly constructed from <input>
    Then strict FPS construction fails with "<message>"

    Examples:
      | input | message                              |
      | 0     | Recording FPS must be greater than 0 |
      | 241   | Recording FPS must be 240 or less    |

  Scenario Outline: Width and Height produce valid H.264 output dimensions
    When recording dimensions <width> x <height> are validated
    Then output dimensions are <output_width> x <output_height> with <rgba_bytes> RGBA bytes

    Examples:
      | width | height | output_width | output_height | rgba_bytes |
      | 1920  | 1080   | 1920         | 1080          | 8294400    |
      | 1921  | 1079   | 1922         | 1080          | 8303040    |
      | 1     | 1      | 2            | 2             | 16         |

  Scenario Outline: Zero Width or Height is rejected before recording
    When recording dimensions <width> x <height> are validated
    Then recording dimensions fail with "Recording region must have non-zero size"

    Examples:
      | width | height |
      | 0     | 1080   |
      | 1920  | 0      |

  Scenario: A Width that cannot be rounded safely is rejected
    When recording dimensions 4294967295 x 2 are validated
    Then recording dimensions fail with "Recording dimension is too large to round to an even value"

  Scenario Outline: Exact frame timing does not accumulate drift
    When timing for frame <frame> at <fps> FPS is computed
    Then the frame timestamp is <timestamp> and duration is <duration> hundred-nanosecond units

    Examples:
      | frame | fps | timestamp | duration |
      | 0     | 30  | 0         | 333333   |
      | 1     | 30  | 333333    | 333333   |
      | 2     | 30  | 666666    | 333334   |
      | 29    | 30  | 9666666   | 333334   |

  Scenario Outline: Recording bitrate is derived and clamped
    When bitrate for <width> x <height> at <fps> FPS is computed
    Then the recording bitrate is <bitrate> bits per second

    Examples:
      | width | height | fps | bitrate  |
      | 100   | 100    | 30  | 2000000  |
      | 1920  | 1080   | 30  | 7464960  |
      | 7680  | 4320   | 120 | 50000000 |
