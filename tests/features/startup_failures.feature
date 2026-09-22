Feature: Auto-start failures and rollback
  Scenario Outline: Failed installation stops before later side effects
    When first-run auto-start is "enabled" with "<failure>" failing
    Then first-run operations are "<operations>"
    And first-run startup reports "<message>"
    Examples:
      | failure  | operations                                   | message                              |
      | none     | inspect,create,copy,register,launch           | handoff                              |
      | inspect  | inspect                                      | Failed to inspect RustShot           |
      | create   | inspect,create                               | Failed to create RustShot            |
      | copy     | inspect,create,copy                          | Failed to copy RustShot              |
      | register | inspect,create,copy,register                 | register denied                      |
      | launch   | inspect,create,copy,register,launch,rollback | Failed to launch installed RustShot  |
      | rollback | inspect,create,copy,register,launch,rollback | additionally failed to remove        |

  Scenario Outline: Disabling auto-start only removes its registration
    When first-run auto-start is "disabled" with "<failure>" failing
    Then first-run operations are "remove"
    And first-run startup reports "<message>"
    Examples:
      | failure | message       |
      | none    | continue      |
      | remove  | remove denied |
