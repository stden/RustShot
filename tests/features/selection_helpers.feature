Feature: Selection overlay helper functions
  Low-level helpers used by the selection overlay: packing screenshot pixels
  into softbuffer format, computing scale factors, converting coordinate
  spaces, and computing incremental surface resize requests.

  # --- pack_rgba_to_softbuffer ---

  Scenario: Single RGBA pixel is packed to 0x00RRGGBB
    Given a 1-pixel RGBA buffer with bytes (10, 20, 30, 255)
    When the RGBA buffer is packed to softbuffer format
    Then the packed buffer has 1 entry
    And the first packed entry is 0x000A141E

  Scenario: Alpha channel is dropped when packing
    Given a 1-pixel RGBA buffer with bytes (255, 0, 0, 128)
    When the RGBA buffer is packed to softbuffer format
    Then the first packed entry is 0x00FF0000

  Scenario: Empty RGBA buffer packs to an empty buffer
    Given an empty RGBA pixel buffer
    When the RGBA buffer is packed to softbuffer format
    Then the packed buffer has 0 entries

  Scenario: Trailing incomplete pixel is ignored when packing
    Given a 5-byte RGBA buffer
    When the RGBA buffer is packed to softbuffer format
    Then the packed buffer has 1 entry

  # --- virtual_desktop_scale_factor ---

  Scenario: Primary monitor scale is used when present
    Given the virtual desktop monitors are [(false, 1.0), (true, 1.5), (false, 2.0)]
    When the virtual desktop scale factor is computed
    Then the scale factor is 1.5

  Scenario: First monitor is the fallback when no primary
    Given the virtual desktop monitors are [(false, 2.0), (false, 1.0)]
    When the virtual desktop scale factor is computed
    Then the scale factor is 2.0

  Scenario: Single monitor returns its scale
    Given the virtual desktop monitors are [(true, 1.25)]
    When the virtual desktop scale factor is computed
    Then the scale factor is 1.25

  Scenario: Empty monitor list returns no scale factor
    Given the virtual desktop monitors are []
    When the virtual desktop scale factor is computed
    Then no scale factor exists

  # --- logical_to_physical ---

  Scenario: Scale factor 1.0 leaves the value unchanged
    Given the logical value is 1920 at scale factor 1.0
    When the logical value is converted to physical pixels
    Then the physical pixel value is 1920

  Scenario: Scale factor 1.5 multiplies and rounds to nearest
    Given the logical value is 1000 at scale factor 1.5
    When the logical value is converted to physical pixels
    Then the physical pixel value is 1500

  Scenario: Negative logical value is scaled correctly
    Given the logical value is -100 at scale factor 2.0
    When the logical value is converted to physical pixels
    Then the physical pixel value is -200

  # --- next_surface_size ---

  Scenario: First call with no current size triggers a resize
    Given the current surface size is not set
    When the next surface size of 1920 x 1080 is requested
    Then a surface resize to 1920 x 1080 is needed

  Scenario: Same size as current does not trigger a resize
    Given the current surface size is 1920 x 1080
    When the next surface size of 1920 x 1080 is requested
    Then no surface resize is needed

  Scenario: Different size triggers a resize
    Given the current surface size is 1920 x 1080
    When the next surface size of 2560 x 1440 is requested
    Then a surface resize to 2560 x 1440 is needed

  Scenario: Zero width is clamped to 1
    Given the current surface size is not set
    When the next surface size of 0 x 100 is requested
    Then a surface resize to 1 x 100 is needed

  # --- find_screen_index_at_position ---

  Scenario: Cursor inside the only screen returns index 0
    Given the screen layout is [(0, 0, 1920, 1080)]
    When the screen at cursor position (960, 540) is searched
    Then screen index 0 is found

  Scenario: Cursor outside all screens returns no result
    Given the screen layout is [(0, 0, 1920, 1080)]
    When the screen at cursor position (5000, 5000) is searched
    Then no screen is found at that position

  Scenario: Cursor on the second of two screens returns index 1
    Given the screen layout is [(0, 0, 1920, 1080), (1920, 0, 1920, 1080)]
    When the screen at cursor position (2500, 300) is searched
    Then screen index 1 is found

  Scenario: Cursor exactly at origin of the first screen returns index 0
    Given the screen layout is [(0, 0, 1920, 1080)]
    When the screen at cursor position (0, 0) is searched
    Then screen index 0 is found

  # --- preferred_screen_index ---

  Scenario: Cursor on second screen wins over primary
    Given the preferred screen layout is [(0, 0, 1920, 1080, true), (1920, 0, 1920, 1080, false)]
    When the preferred screen for cursor at (2500, 300) is determined
    Then the preferred screen index is 1

  Scenario: No cursor falls back to primary screen
    Given the preferred screen layout is [(0, 0, 1920, 1080, false), (1920, 0, 1920, 1080, true)]
    When the preferred screen with no cursor is determined
    Then the preferred screen index is 1

  Scenario: No cursor no primary falls back to first screen
    Given the preferred screen layout is [(0, 0, 1920, 1080, false), (1920, 0, 1920, 1080, false)]
    When the preferred screen with no cursor is determined
    Then the preferred screen index is 0

  Scenario: Empty screen list returns no preferred screen
    Given the preferred screen layout is []
    When the preferred screen with no cursor is determined
    Then no preferred screen exists
