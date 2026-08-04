Feature: Settings JSON persistence requirements
  The hand-written JSON codec must preserve all five settings fields, escape
  paths correctly, emit each schema key once, and recover safely from missing
  or incorrectly typed hand-edited values.

  @settings-12.1 @settings-17.5
  Scenario: Every settings field survives a serialize and parse roundtrip
    Given application settings:
      | screenshots_dir | recordings_dir | format | fps | auto_start |
      | /custom/截图     | /custom/录像    | bmp    | 120 | false      |
    When the settings are serialized and parsed
    Then every settings field matches the original
    And the serialized settings are valid JSON

  Scenario: Quotes and backslashes in paths survive a roundtrip
    Given application settings:
      | screenshots_dir       | recordings_dir         | format | fps | auto_start |
      | C:\\Users\\"test\\Shots | D:\\Videos\\Recordings | jpg    | 24  | true       |
    When the settings are serialized and parsed
    Then every settings field matches the original
    And the serialized settings are valid JSON

  @settings-12.7
  Scenario: The serializer emits every schema key exactly once
    Given default application settings
    When the settings are serialized
    Then every settings schema key occurs exactly once

  Scenario: Missing fields keep their defaults
    When settings JSON is parsed:
      """
      {"default_format":"jpg"}
      """
    Then the parsed settings format is "jpg"
    And the other parsed settings fields equal their defaults

  Scenario: Incorrect JSON value types keep their defaults
    When settings JSON is parsed:
      """
      {"screenshots_dir":null,"recordings_dir":false,"recording_fps":"60","auto_start":"false"}
      """
    Then all parsed settings fields equal their defaults

  Scenario: Duplicate fields use the first occurrence
    When settings JSON is parsed:
      """
      {"default_format":"bmp","default_format":"jpg","recording_fps":60,"recording_fps":120,"auto_start":false,"auto_start":true}
      """
    Then the parsed settings format is "bmp"
    And the parsed recording FPS is 60
    And parsed auto-start is false
