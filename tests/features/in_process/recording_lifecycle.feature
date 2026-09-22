@in_process @tray_start_record
Feature: Recording session lifecycle
  These scenarios control the worker outcome so stop/error handling is
  deterministic. Native encoding and pixel conversion have separate coverage.

  Scenario Outline: Stop waits for finalization and returns the worker outcome
    Given a controlled recording worker will "<outcome>" when stopped
    When the recording session is stopped
    Then recording shutdown waits for the worker to finish
    And recording completion reports "<result>"
    Examples:
      | outcome  | result                     |
      | finalize | recordings/session.mp4     |
      | fail     | encoder failed             |
      | panic    | Recording thread panicked  |

  Scenario: Releasing a recording owner still signals and joins its worker
    Given a controlled recording worker will "finalize" when stopped
    When the recording session owner is dropped
    Then recording shutdown waits for the worker to finish
