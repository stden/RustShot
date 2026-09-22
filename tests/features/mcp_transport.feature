Feature: MCP stdio sessions
  Scenario: Multiple requests survive malformed input and terminate on EOF
    When the RustShot MCP process receives this transcript before EOF:
      """
      {"jsonrpc":"2.0","id":"init","method":"initialize"}
      {"jsonrpc":"2.0","method":"notifications/initialized"}
      not json
      {"jsonrpc":"2.0","id":2,"method":"ping"}
      {"jsonrpc":"2.0","id":3,"method":"tools/list"}
      """
    Then the MCP process exits successfully with 4 replies
    And the ordered MCP replies contain:
      | index | pointer                 | value         |
      | 0     | /id                     | "init"        |
      | 0     | /result/serverInfo/name | "rustshot"    |
      | 1     | /id                     | null          |
      | 1     | /error/code             | -32700        |
      | 2     | /id                     | 2             |
      | 2     | /result                 | {}            |
      | 3     | /result/tools/0/name    | "list_screens"|

  Scenario: A notification-only session emits no protocol output
    When the RustShot MCP process receives this transcript before EOF:
      """
      {"jsonrpc":"2.0","method":"notifications/initialized"}
      {"jsonrpc":"2.0","method":"ping"}
      """
    Then the MCP process exits successfully with 0 replies

  @mcp_list_screens
  Scenario: Display enumeration is available to MCP clients
    When the MCP tool "list_screens" is called
    Then the MCP reply at "/result/isError" is false
    And the MCP reply text at "/result/content/0/text" contains "Available screens:"

  @mcp_list_windows
  Scenario: Window enumeration is available on every platform
    When the MCP tool "list_windows" is called
    Then the MCP reply at "/result/isError" is false
    And the MCP reply at "/result/content/0/type" is "text"

  Scenario: CLI listing options return without opening a selector
    When the executable lists its available screens and windows
    Then both listing commands finish without starting a capture
