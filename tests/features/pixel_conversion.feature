Feature: Pixel format conversion requirements
  Windows encoders consume bottom-up BGRA while screenshots expose top-down
  RGBA. Conversion swaps red and blue, reverses rows, preserves alpha, and
  rejects malformed buffers without panicking.

  @recording-9.1 @recording-9.2 @recording-9.4
  Scenario: A two-row RGBA frame is channel-swapped and vertically flipped
    When RGBA pixels "1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16" sized 2 x 2 are converted to bottom-up BGRA
    Then the converted pixels are "11,10,9,12,15,14,13,16,3,2,1,4,7,6,5,8"

  @recording-9.3 @recording-17.4
  Scenario: Applying the conversion twice restores the original frame
    When RGBA pixels "10,20,30,40,50,60,70,80" sized 1 x 2 are converted twice
    Then the converted pixels are "10,20,30,40,50,60,70,80"

  @recording-9.5
  Scenario: Alpha values stay attached to their pixels after the row flip
    When RGBA pixels "10,20,30,0,40,50,60,128,70,80,90,200,100,110,120,255" sized 2 x 2 are converted to bottom-up BGRA
    Then the converted alpha values are "200,255,0,128"

  Scenario Outline: A zero-sized frame converts to an empty buffer
    When empty RGBA pixels sized <width> x <height> are converted to bottom-up BGRA
    Then the converted pixel buffer is empty

    Examples:
      | width | height |
      | 0     | 100    |
      | 100   | 0      |

  Scenario: Screenshot BGRA conversion swaps channels without changing alpha
    When screenshot BGRA pixels "3,2,1,4,7,6,5,8" are converted to RGBA
    Then the converted pixels are "1,2,3,4,5,6,7,8"

  @recording-9.6 @recording-9.7 @recording-9.8
  Scenario Outline: Packed Media Foundation pairs preserve both u32 values
    When values <high> and <low> are packed into one u64
    Then unpacking yields <high> and <low>

    Examples:
      | high       | low        |
      | 1920       | 1080       |
      | 4294967295 | 0          |
      | 0          | 4294967295 |
      | 4294967295 | 4294967295 |
