Feature: Image format resolution from a file extension
  When the user supplies an output path, RustShot derives the image format from
  its extension. Matching is case-insensitive, "jpeg" is an alias for JPG, and
  any unrecognised extension yields no format (the caller falls back elsewhere).

  Scenario Outline: A known extension maps to a format
    When the format for extension "<ext>" is resolved
    Then the resolved format is "<format>"

    Examples:
      | ext  | format |
      | png  | Png    |
      | PNG  | Png    |
      | jpg  | Jpg    |
      | JPG  | Jpg    |
      | jpeg | Jpg    |
      | JPEG | Jpg    |
      | bmp  | Bmp    |
      | BMP  | Bmp    |

  Scenario Outline: An unrecognised extension has no format
    When the format for extension "<ext>" is resolved
    Then no format is resolved

    Examples:
      | ext  |
      | webp |
      | gif  |
      | txt  |
      |      |

  Scenario Outline: The effective format prefers the explicit flag, then the output extension, then settings
    Given the CLI format "<cli>" was given <how>
    And the settings format is "<settings>"
    And the output path is "<output>"
    When the effective CLI format is resolved
    Then the resolved format is "<effective>"

    Examples:
      | cli | how        | settings | output   | effective |
      | jpg | explicitly | png      | shot.bmp | Jpg       |
      | jpg | implicitly | png      | shot.bmp | Bmp       |
      | jpg | implicitly | bmp      | shot.xyz | Bmp       |
      | jpg | implicitly | bmp      |          | Bmp       |
      | png | implicitly | jpg      | shot     | Jpg       |

  Scenario Outline: An unsupported output extension is surfaced for the error message
    Given the output path is "<output>"
    When the output path is checked for an unsupported extension
    Then the reported unsupported extension is <reported>

    Examples:
      | output    | reported |
      | shot.gif  | "gif"    |
      | shot.webp | "webp"   |
      | shot.png  | nothing  |
      | shot      | nothing  |
      |           | nothing  |
