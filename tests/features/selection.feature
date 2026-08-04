Feature: Building a selection region from a mouse drag
  Dragging a rectangle on the selection overlay produces a canonical region.
  The rectangle is normalized so drag direction does not matter, and a drag is
  rejected unless both sides exceed 5 pixels (guards against accidental clicks).

  Scenario: A normal drag produces a region
    Given a drag from (10, 20) to (110, 220)
    When the drag region is built
    Then a region is produced at (10, 20) sized 100 x 200

  Scenario: A reversed drag produces the same canonical region
    Given a drag from (110, 220) to (10, 20)
    When the drag region is built
    Then a region is produced at (10, 20) sized 100 x 200

  Scenario: A drag of exactly 5 pixels per side is rejected
    Given a drag from (0, 0) to (5, 5)
    When the drag region is built
    Then no region is produced

  Scenario: A drag wide enough but not tall enough is rejected
    Given a drag from (0, 0) to (100, 5)
    When the drag region is built
    Then no region is produced

  Scenario: A drag tall enough but not wide enough is rejected
    Given a drag from (0, 0) to (5, 100)
    When the drag region is built
    Then no region is produced

  Scenario: A drag just over the threshold is accepted
    Given a drag from (0, 0) to (6, 6)
    When the drag region is built
    Then a region is produced at (0, 0) sized 6 x 6

  @selection-3.1
  Scenario: An identical drag start and end is rejected
    Given a drag from (500, 300) to (500, 300)
    When the drag region is built
    Then no region is produced

  @selection-3.2
  Scenario: A drag spanning negative coordinates is normalized
    Given a drag from (-50, -30) to (60, 80)
    When the drag region is built
    Then a region is produced at (-50, -30) sized 110 x 110
