Feature: Physical layout of gap-separated monitors
  The OS reports monitor positions in logical (DPI-scaled) coordinates. To
  crop and record correctly, RustShot rebuilds the arrangement in physical
  pixels: the primary monitor anchors the origin (0, 0), and a monitor that
  touches no other is placed relative to its nearest already-placed
  neighbour, with the gap between them rescaled. At 100% scale the physical
  arrangement must reproduce the logical one, translated so the primary
  monitor sits at the origin.

  Scenario: A row of gap-separated monitors keeps its gaps at 100% scale
    Given the monitor layout:
      | logical_x | logical_y | width | height | primary |
      | 0         | 0         | 1000  | 1000   | yes     |
      | 1500      | 0         | 1000  | 1000   | no      |
      | 3000      | 0         | 1000  | 1000   | no      |
    When the physical screen layout is built
    Then the physical layout is:
      | x    | y | width | height |
      | 0    | 0 | 1000  | 1000   |
      | 1500 | 0 | 1000  | 1000   |
      | 3000 | 0 | 1000  | 1000   |

  Scenario: The nearest left neighbour wins when two placed monitors compete
    Given the monitor layout:
      | logical_x | logical_y | width | height | primary |
      | 1500      | 0         | 1000  | 1000   | yes     |
      | 0         | 0         | 1000  | 1000   | no      |
      | 3000      | 0         | 1000  | 1000   | no      |
    When the physical screen layout is built
    Then the physical layout is:
      | x     | y | width | height |
      | 0     | 0 | 1000  | 1000   |
      | -1500 | 0 | 1000  | 1000   |
      | 1500  | 0 | 1000  | 1000   |

  Scenario: The nearest right neighbour wins when two placed monitors compete
    Given the monitor layout:
      | logical_x | logical_y | width | height | primary |
      | 1500      | 0         | 1000  | 1000   | yes     |
      | 3000      | 0         | 1000  | 1000   | no      |
      | 0         | 0         | 1000  | 1000   | no      |
    When the physical screen layout is built
    Then the physical layout is:
      | x     | y | width | height |
      | 0     | 0 | 1000  | 1000   |
      | 1500  | 0 | 1000  | 1000   |
      | -1500 | 0 | 1000  | 1000   |

  Scenario: The nearest upper neighbour wins in a column of monitors
    Given the monitor layout:
      | logical_x | logical_y | width | height | primary |
      | 0         | 1500      | 1000  | 1000   | yes     |
      | 0         | 0         | 1000  | 1000   | no      |
      | 0         | 3000      | 1000  | 1000   | no      |
    When the physical screen layout is built
    Then the physical layout is:
      | x | y     | width | height |
      | 0 | 0     | 1000  | 1000   |
      | 0 | -1500 | 1000  | 1000   |
      | 0 | 1500  | 1000  | 1000   |

  Scenario: The nearest lower neighbour wins in a column of monitors
    Given the monitor layout:
      | logical_x | logical_y | width | height | primary |
      | 0         | 1500      | 1000  | 1000   | yes     |
      | 0         | 3000      | 1000  | 1000   | no      |
      | 0         | 0         | 1000  | 1000   | no      |
    When the physical screen layout is built
    Then the physical layout is:
      | x | y     | width | height |
      | 0 | 0     | 1000  | 1000   |
      | 0 | 1500  | 1000  | 1000   |
      | 0 | -1500 | 1000  | 1000   |

  Scenario: A monitor reporting a non-positive scale factor is rejected
    When screen layout inputs are built for captured screen 0 sized 1920x1080 from:
      | logical_x | logical_y | width | height | scale | primary |
      | 0         | 0         | 1920  | 1080   | 1.0   | yes     |
      | 1920      | 0         | 1280  | 720    | 0.0   | no      |
    Then building the layout inputs fails with "Display scale factor must be positive"

  Scenario: The captured screen uses the frame size, the others their scaled logical size
    When screen layout inputs are built for captured screen 0 sized 1920x1080 from:
      | logical_x | logical_y | width | height | scale | primary |
      | 0         | 0         | 1920  | 1080   | 1.0   | yes     |
      | 1920      | 0         | 1280  | 720    | 1.5   | no      |
    Then the computed physical sizes are:
      | width | height |
      | 1920  | 1080   |
      | 1920  | 1080   |
