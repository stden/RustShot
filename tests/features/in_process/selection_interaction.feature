@in_process @tray_capture
Feature: Interactive selection workflows
  Background:
    Given a frozen 100 by 100 screenshot in the selection overlay

  Scenario Outline: Dragging in either direction selects the same physical pixels
    When the pointer drags from (<x>, <y>) to (<end_x>, <end_y>)
    Then selection completes at (10, 20) sized 50 by 50
    Examples:
      | x  | y  | end_x | end_y |
      | 10 | 20 | 60    | 70    |
      | 60 | 70 | 10    | 20    |

  Scenario: A too-small drag allows a second attempt
    When the pointer drags from (10, 10) to (13, 13)
    Then the overlay remains ready for another drag
    When the pointer drags from (20, 20) to (50, 60)
    Then selection completes at (20, 20) sized 30 by 40

  Scenario: Cancellation cannot turn the button release into a capture
    When the overlay is cancelled during a drag
    Then no selected region is returned after cancellation

  Scenario: Resizing invalidates an unfinished drag
    When the overlay is resized during a drag and the button is released
    Then the overlay remains ready for another drag

  Scenario: Stopping recording also dismisses its pending selector
    When the recording parent writes a stop marker before a drag completes
    Then no selected region is returned after cancellation

  Scenario Outline: Region screenshots go only to the clipboard
    When a completed region is delivered with clipboard "<clipboard>"
    Then region delivery reports "<result>" without writing a file
    Examples:
      | clipboard   | result         |
      | available   | copied         |
      | unavailable | clipboard busy |
