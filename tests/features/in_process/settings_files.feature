@in_process
Feature: Settings files and external editing
  Background:
    Given an isolated settings file and a recording editor launcher

  Scenario: A first launch reads defaults without writing settings
    When settings are loaded from disk
    Then the default settings are returned without creating a file

  Scenario: Saving persists every configurable field
    When custom folders, JPG, 60 FPS, and disabled auto-start are saved
    And settings are loaded from disk
    Then all saved settings are loaded and the file is valid JSON
    And no settings scratch file remains

  Scenario: A later save replaces the previous settings completely
    Given a previous settings file exists
    When custom folders, JPG, 60 FPS, and disabled auto-start are saved
    And settings are loaded from disk
    Then all saved settings are loaded and the file is valid JSON
    And no settings scratch file remains

  Scenario: A blocked config folder produces a readable save error
    Given the settings parent is blocked by an existing file
    When custom folders, JPG, 60 FPS, and disabled auto-start are saved
    Then saving settings fails mentioning "Failed to create config dir"
    And the blocked parent file is preserved

  Scenario: A failed replacement cleans up its scratch file
    Given a directory occupies the settings filename
    When custom folders, JPG, 60 FPS, and disabled auto-start are saved
    Then saving settings fails mentioning "Failed to write settings"
    And no settings scratch file remains

  @tray_settings
  Scenario: Opening settings for editing creates defaults on first use
    When the settings editor is requested
    Then a default settings file is ready for editing
    And the editor receives the isolated settings path once

  Scenario: Opening malformed settings preserves the user's hand edits
    Given the existing settings file contains malformed hand edits
    When the settings editor is requested
    Then the hand-edited settings bytes are unchanged
    And the editor receives the isolated settings path once

  Scenario: An editor is not opened for a settings file that cannot be created
    Given the settings parent is blocked by an existing file
    When the settings editor is requested
    Then the editor is not launched
    And the blocked parent file is preserved
