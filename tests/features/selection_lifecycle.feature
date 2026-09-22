Feature: Selection cancellation and overlapping requests
  Scenario Outline: A running selector prevents overlapping selection windows
    When a selection child reports "<state>"
    Then another selection is <next>
    Examples:
      | state             | next    |
      | running           | blocked |
      | finished          | allowed |
      | inspection failed | allowed |

  Scenario Outline: Cancellation is quiet and failures remain visible
    When selection finishes with "<result>"
    Then the selection diagnostic is "<diagnostic>"
    Examples:
      | result                       | diagnostic                   |
      | Selection cancelled          |                              |
      | Selection cancelled by error | Selection cancelled by error |
      | capture failed               | capture failed               |
