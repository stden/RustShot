Feature: Screenshot and recording output files
  Background:
    Given an isolated screenshot output directory

  Scenario Outline: Every supported encoder produces an image viewers can read
    When a known image is saved as "<format>"
    Then the saved file has the requested encoding, dimensions, and pixels
    Examples:
      | format |
      | png    |
      | jpg    |
      | bmp    |

  Scenario: An invalid parent cannot destroy an existing file
    When an image is saved beneath a file instead of a directory
    Then saving reports the file error and preserves the existing file

  Scenario: Recording names are safe and stay in the configured folder
    When a recording filename is requested in the isolated directory
    Then the recording path is a unique timestamped MP4 in that directory
