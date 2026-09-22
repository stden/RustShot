Feature: Output file path resolution
  An explicit output path is used verbatim — RustShot never rewrites what the
  user asked for, even if the extension disagrees with the format. Without an
  explicit path, a timestamped "screenshot_<timestamp>.<ext>" name is generated
  inside the screenshots directory, with the extension taken from the format.

  Scenario: An explicit output path is used verbatim
    When the output path "shots/custom.jpg" is resolved for format "png" in directory "elsewhere"
    Then the resolved path is "shots/custom.jpg"

  Scenario Outline: A missing output path generates a timestamped name in the screenshots directory
    When an output path is generated for format "<format>" in directory "shots"
    Then the generated file is in "shots" and its name starts with "screenshot_" and ends with ".<format>"

    Examples:
      | format |
      | png    |
      | jpg    |
      | bmp    |
