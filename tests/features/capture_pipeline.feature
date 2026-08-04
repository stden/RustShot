Feature: Screenshot capture pipeline requirements
  The screenshot pipeline runs capture, directory preparation, and encoding in
  order. A failed stage stops every later stage, generated file names are safe
  and predictable, and directory creation is idempotent.

  @capture-1.7
  Scenario: A capture failure skips directory creation and encoding
    When the screenshot pipeline is run with "capture" failing as "hardware failure"
    Then the screenshot pipeline fails with "hardware failure"
    And the screenshot pipeline stages are "capture"

  @capture-2.4
  Scenario: A directory failure skips image encoding
    When the screenshot pipeline is run with "directory" failing as "permission denied"
    Then the screenshot pipeline fails with "permission denied"
    And the screenshot pipeline stages are "capture,directory"

  @capture-2.2 @capture-2.6
  Scenario: Generated screenshot names use the canonical safe timestamp
    When an output path is generated for format "png" in directory "shots"
    Then the generated screenshot timestamp has the canonical safe shape

  @capture-2.1
  Scenario: The default screenshot directory is named RustShot
    When a default output path is generated for format "png"
    Then the generated file parent directory is named "RustShot"

  @capture-2.3
  Scenario: An explicit path containing spaces is preserved verbatim
    When the output path "path with spaces/my screenshot.png" is resolved for format "jpg" in directory "elsewhere"
    Then the resolved path is "path with spaces/my screenshot.png"

  @capture-2.4 @capture-2.5
  Scenario: Creating a nested output directory is idempotent
    When the same nested screenshot parent directory is prepared three times
    Then all screenshot parent directory preparations succeed

  Scenario Outline: Every supported capture format maps to its encoder
    When capture format "<format>" is mapped to an image encoder
    Then the image encoder is "<encoder>"

    Examples:
      | format | encoder |
      | png    | Png     |
      | jpg    | Jpeg    |
      | bmp    | Bmp     |
