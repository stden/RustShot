Feature: Discovering windows to capture
  Scenario Outline: Only visible titled windows are offered to the user
    When a <visibility> window titled "<title>" is considered for listing
    Then the window is <decision>
    Examples:
      | visibility | title        | decision |
      | visible    | Editor       | included |
      | hidden     | Background   | excluded |
      | visible    |              | excluded |
      | visible    | スクリーン   | included |

  Scenario: Window IDs and Unicode titles survive listing
    When these windows are formatted for the user:
      | id                   | title      |
      | 42                   | Editor     |
      | 18446744073709551615 | スクリーン |
    Then the listing is:
      """
      Open windows:
        42  Editor
        18446744073709551615  スクリーン
      """

  Scenario: An empty desktop has an explicit message
    When an empty window list is formatted for the user
    Then the listing is:
      """
      No open windows found.
      """
