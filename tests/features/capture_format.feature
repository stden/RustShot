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
