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

  # Everything above drives the two-buffer form, which no production code calls
  # any more. The screenshot path converts inside the captured buffer instead, to
  # avoid allocating and first-touching a second monitor-sized buffer per
  # screenshot. The flip pairs row y with row height - 1 - y and the swizzle only
  # trades red for blue, so the whole transform is its own inverse and needs no
  # destination at all.

  @recording-9.1 @recording-9.2 @recording-9.4
  Scenario: An even-height frame is swizzled and flipped inside its own buffer
    # Same bytes and same expectation as the two-buffer 2 x 2 scenario above: the
    # two forms are only interchangeable while they agree byte for byte.
    When RGBA pixels "1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16" sized 2 x 2 are flipped and swizzled in place
    Then the converted pixels are "11,10,9,12,15,14,13,16,3,2,1,4,7,6,5,8"

  @recording-9.3 @recording-17.4
  Scenario: Flipping and swizzling in place twice restores the original frame
    # The previous scenario's output fed straight back in. This is why the
    # screenshot path may run the very same call in the opposite direction --
    # bottom-up BGRA in, top-down RGBA out -- and get byte-identical results.
    When RGBA pixels "11,10,9,12,15,14,13,16,3,2,1,4,7,6,5,8" sized 2 x 2 are flipped and swizzled in place
    Then the converted pixels are "1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16"

  @recording-9.1 @recording-9.2
  Scenario: An odd-height frame swizzles its unpaired middle row exactly once
    # 2 x 3: the middle row is the one row with no partner to swap with, and so the
    # one an in-place flip gets wrong. Skipping it and swizzling it twice are
    # indistinguishable, because the swizzle is its own inverse; both leave the
    # middle row as "9,10,11,12,13,14,15,16" -- one scanline with red and blue
    # crossed in an otherwise perfect screenshot, which no check on buffer length
    # or corner pixels would notice. The channel values are deliberately
    # asymmetric: a grey row looks identical swizzled and unswizzled.
    When RGBA pixels "1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24" sized 2 x 3 are flipped and swizzled in place
    Then the converted pixels are "19,18,17,20,23,22,21,24,11,10,9,12,15,14,13,16,3,2,1,4,7,6,5,8"

  Scenario: A single-row frame keeps its left-to-right pixel order
    # Height 1 forms no pair at all, so the only row is the unpaired one. A flip
    # that reversed bytes instead of whole rows would pass every one-pixel-wide
    # scenario and fail here.
    When RGBA pixels "1,2,3,4,5,6,7,8,9,10,11,12" sized 3 x 1 are flipped and swizzled in place
    Then the converted pixels are "3,2,1,4,7,6,5,8,11,10,9,12"

  Scenario Outline: A zero-sized frame is flipped and swizzled to nothing
    # Empty quotes are a frame with no bytes. Zero width makes the row length zero,
    # which is why the row loop is indexed rather than chunked: chunks_exact_mut(0)
    # panics, and this form must accept the same zero-sized frames the two-buffer
    # one accepts.
    When RGBA pixels "" sized <width> x <height> are flipped and swizzled in place
    Then the converted pixel buffer is empty

    Examples:
      | width | height |
      | 0     | 100    |
      | 100   | 0      |

  Scenario: A buffer shorter than its dimensions is rejected with both byte counts
    # 12 bytes offered for a 16-byte frame. One buffer, so the message names no
    # direction -- it is the caller's own frame that is short -- and it carries both
    # counts, because a caller told only "too short" cannot tell by how much.
    When RGBA pixels "1,2,3,4,5,6,7,8,9,10,11,12" sized 2 x 2 are flipped and swizzled in place
    Then the conversion fails mentioning "Pixel buffer is too short: expected 16 bytes, got 12"

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
