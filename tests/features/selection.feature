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

  Scenario: A drag just over the threshold is accepted
    Given a drag from (0, 0) to (6, 6)
    When the drag region is built
    Then a region is produced at (0, 0) sized 6 x 6

  Scenario: Cropping a packed region extracts RGBA pixels with full opacity
    Given a 2 x 2 packed background buffer
    When a region at (1, 0) sized 1 x 1 is cropped from the packed buffer
    Then the cropped image is 1 x 1 with top-left pixel (32, 64, 96, 255)

  Scenario: Cropping a packed region with negative coordinates fails
    Given a 2 x 2 packed background buffer
    When a region at (-1, 0) sized 1 x 1 is cropped from the packed buffer
    Then cropping fails with "Selection region has negative coordinates"

  Scenario: Cropping an out-of-bounds packed region fails
    Given a 2 x 2 packed background buffer
    When a region at (1, 1) sized 2 x 1 is cropped from the packed buffer
    Then cropping fails with "Selection region is out of bounds"

  Scenario: Overlay damage rect covers the full surface on the first frame
    When overlay damage rects are computed for a new 100 x 80 surface at cursor (10, 20)
    Then 1 overlay damage rect is produced

  Scenario: Overlay damage rects are empty when the cursor does not move
    When overlay damage rects are computed for an unchanged 100 x 80 surface at cursor (10, 20)
    Then 0 overlay damage rects are produced

  Scenario: Moving the crosshair produces damage rects for both old and new positions
    When overlay damage rects are computed for a 100 x 80 surface with cursor moving from (10, 20) to (30, 40)
    Then 4 overlay damage rects are produced

  Scenario: Bottom border is drawn when drag end is beyond the buffer height
    When a 100 x 10 selection overlay is rendered with drag from (0, 0) to (50, 15)
    Then the last row of the rendered buffer has at least one non-zero pixel

  Scenario: Right border is drawn when drag end is beyond the buffer width
    When a 100 x 10 selection overlay is rendered with drag from (0, 0) to (105, 5)
    Then the last column of the rendered buffer has at least one non-zero pixel
