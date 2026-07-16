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

  Scenario Outline: An out-of-range recording FPS falls back to the default
    The Fps newtype only ever holds a value in 1..=240, so a hand-edited
    settings.json cannot smuggle an invalid rate past parsing: anything outside
    the range becomes the default (30) rather than failing later at record time.

    When the recording FPS <input> is normalized
    Then the normalized recording FPS is <result>

    Examples:
      | input | result |
      | 30    | 30     |
      | 60    | 60     |
      | 240   | 240    |
      | 0     | 30     |
      | 241   | 30     |
      | 300   | 30     |
