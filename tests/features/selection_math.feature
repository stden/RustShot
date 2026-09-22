Feature: DPI coordinate and interval requirements
  Monitor layout uses half-open intervals and explicit DPI scaling. Touching
  ranges do not overlap, fractional scales are rounded consistently, and
  negative desktop coordinates remain meaningful.

  @selection-1.1 @selection-1.2 @selection-1.3 @selection-17.2
  Scenario Outline: Half-open ranges have deterministic overlap semantics
    When range <a_start> sized <a_size> is compared with range <b_start> sized <b_size>
    Then the ranges <verdict> overlap

    Examples:
      | a_start | a_size | b_start | b_size | verdict |
      | 10      | 0      | 20      | 0      | do not  |
      | 50      | 0      | 0       | 100    | do not  |
      | 0       | 100    | 50      | 0      | do not  |
      | 0       | 10     | 10      | 10     | do not  |
      | 0       | 10     | 9       | 10     | do      |
      | -10     | 20     | 0       | 5      | do      |

  @selection-1.4 @selection-1.5
  Scenario Outline: Monitor scale is physical pixels divided by logical pixels
    When the monitor scale ratio for <logical> logical and <physical> physical pixels is computed
    Then the monitor scale ratio is <ratio>

    Examples:
      | logical | physical | ratio |
      | 100     | 250      | 2.5   |
      | 1920    | 960      | 0.5   |
      | 0       | 960      | 1.0   |

  @selection-1.6 @selection-1.7
  Scenario Outline: Logical gaps are scaled into physical pixels
    When logical gap <gap> is scaled from <logical> to <physical> pixels
    Then the scaled gap is <result>

    Examples:
      | gap | logical | physical | result |
      | -10 | 3       | 4        | -13    |
      | 100 | 1920    | 2880     | 150    |
      | 7   | 100     | 150      | 11     |
      | 12  | 0       | 300      | 12     |

  @selection-1.8 @selection-1.9 @selection-17.1
  Scenario Outline: Logical coordinates are rounded into physical coordinates
    When logical coordinate <value> is converted at scale <scale>
    Then the physical coordinate is <result>

    Examples:
      | value | scale | result |
      | -7    | 1.25  | -9     |
      | 0     | 3.0   | 0      |
      | 10    | 1.5   | 15     |

  @selection-domain-1
  Scenario: A logical point converts to a distinct physical point
    When logical point (-7, 10) is converted through scale factor 1.25
    Then the typed physical point is (-9, 13)

  @selection-domain-2
  Scenario: A logical size converts to a distinct physical size
    When logical size 1920 x 1080 is converted through scale factor 1.5
    Then the typed physical size is 2880 x 1620

  @selection-domain-3
  Scenario Outline: A scale factor must be positive and finite
    When scale factor "<scale>" is validated
    Then scale factor validation fails with "Display scale factor must be positive"

    Examples:
      | scale     |
      | 0         |
      | -1        |
      | NaN       |
      | +infinity |

  Scenario: A drag can span the full signed desktop coordinate range
    Given a drag from (-2147483648, 2147483647) to (2147483647, -2147483648)
    When the drag region is built
    Then a region is produced at (-2147483648, -2147483648) sized 4294967295 x 4294967295

  Scenario: Monitor lookup handles an edge beyond the signed coordinate range
    Given the physical monitors:
      | x          | y          | width | height | primary |
      | 2147483637 | 2147483637 | 20    | 20     | no      |
      | 0          | 0          | 1920  | 1080   | yes     |
    When the preferred monitor is resolved at cursor position (2147483647, 2147483647)
    Then preferred monitor 0 is selected

  Scenario: Normalization rejects an origin outside signed local coordinates
    Given the resolved physical monitor layout:
      | logical_x | logical_y | logical_width | logical_height | physical_x  | physical_y | physical_width | physical_height |
      | 0         | 0         | 1             | 1              | -2147483648 | 0          | 1              | 1               |
      | 0         | 0         | 1             | 1              | 0           | 0          | 1              | 1               |
    When the resolved physical layout is normalized
    Then normalizing the physical layout fails mentioning "origin is out of range"
