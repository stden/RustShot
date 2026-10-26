Feature: Settings normalization
  Persisted settings may be edited by hand, so RustShot clamps them to safe
  values before use. Unlike extension matching, the stored format string is
  matched case-sensitively against the supported set (png, jpg, bmp).

  Scenario Outline: The default image format is clamped to a supported value
    When the default format "<input>" is normalized
    Then the normalized default format is "<result>"

    Examples:
      | input | result |
      | png   | png    |
      | jpg   | jpg    |
      | bmp   | bmp    |
      | webp  | png    |
      | JPG   | png    |
      |       | png    |

  Scenario Outline: The recording FPS falls back to the default when zero
    When the recording FPS <input> is normalized
    Then the normalized recording FPS is <result>

    Examples:
      | input | result |
      | 30    | 30     |
      | 60    | 60     |
      | 240   | 240    |
      | 0     | 30     |

  Scenario: Settings JSON round-trips escaped paths and clamps zero FPS
    When settings JSON with screenshots_dir "C:\Shots\Sub" and recording_fps 0 is parsed and formatted
    Then the reparsed settings have screenshots_dir "C:\Shots\Sub" and recording_fps 30
