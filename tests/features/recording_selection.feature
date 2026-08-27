Feature: Recording selection must belong to one monitor
  Recording has no cross-monitor compositing path. The selected physical
  rectangle must fit one monitor and is translated to that monitor's local
  coordinates before capture. Cursor targeting uses half-open monitor bounds.

  Background:
    Given the physical monitors:
      | x    | y | width | height | primary |
      | 0    | 0 | 1920  | 1080   | no      |
      | 1920 | 0 | 2560  | 1440   | yes     |

  @selection-3.4
  Scenario: A region spanning two monitors is rejected
    When recording region (1800, 100) sized 300 x 200 is resolved
    Then recording selection fails mentioning "single monitor"

  @selection-3.5
  Scenario: A region exactly filling one monitor is accepted
    When recording region (0, 0) sized 1920 x 1080 is resolved
    Then monitor 0 is selected with local region (0, 0) sized 1920 x 1080

  @selection-3.3
  Scenario: A region on the second monitor becomes screen-local
    When recording region (2020, 50) sized 300 x 200 is resolved
    Then monitor 1 is selected with local region (100, 50) sized 300 x 200

  @selection-3.7
  Scenario: A cursor outside every monitor has no direct match
    When the cursor position (9999, 9999) is resolved against the physical monitors
    Then no cursor monitor is selected

  @selection-3.8
  Scenario: Without a cursor the primary monitor is preferred
    When the preferred monitor is resolved without a cursor
    Then preferred monitor 1 is selected

  @selection-3.6
  Scenario: A cursor hit takes priority over the primary monitor
    When the preferred monitor is resolved at cursor position (100, 100)
    Then preferred monitor 0 is selected
