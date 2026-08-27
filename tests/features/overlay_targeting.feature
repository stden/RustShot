Feature: Overlay monitor targeting
  The selection overlay is pinned fullscreen to the monitor being captured.
  The monitor is matched by its exact origin and size; when DPI virtualisation
  makes the reported size disagree with the captured frame, the origin alone
  still identifies the monitor. With no origin match at all the overlay falls
  back to plain window placement instead of guessing.

  Scenario: The monitor matching origin and size is chosen
    Given the connected monitors:
      | origin_x | origin_y | width | height |
      | 0        | 0        | 1920  | 1080   |
      | 1920     | 0        | 2560  | 1440   |
    When the overlay targets origin (1920, 0) sized 2560 x 1440
    Then overlay monitor 1 is selected

  Scenario: A monitor with a mismatched reported size is still found by its origin
    Given the connected monitors:
      | origin_x | origin_y | width | height |
      | 0        | 0        | 1920  | 1080   |
      | 1920     | 0        | 2048  | 1152   |
    When the overlay targets origin (1920, 0) sized 2560 x 1440
    Then overlay monitor 1 is selected

  Scenario: An exact match beats an earlier origin-only match
    Given the connected monitors:
      | origin_x | origin_y | width | height |
      | 0        | 0        | 1920  | 1080   |
      | 0        | 0        | 2560  | 1440   |
    When the overlay targets origin (0, 0) sized 2560 x 1440
    Then overlay monitor 1 is selected

  Scenario: No matching origin means no overlay monitor
    Given the connected monitors:
      | origin_x | origin_y | width | height |
      | 0        | 0        | 1920  | 1080   |
    When the overlay targets origin (5000, 0) sized 800 x 600
    Then no overlay monitor is selected
