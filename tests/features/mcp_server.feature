Feature: MCP server protocol handling
  RustShot exposes its capture tools over the MCP stdio transport
  (newline-delimited JSON-RPC 2.0), started with `--mcp`. handle_message is the
  pure protocol core: a request gets exactly one reply, a notification gets
  none, malformed input is answered with a JSON-RPC parse error instead of
  crashing, and tool failures are reported as MCP tool errors (isError: true)
  rather than transport faults.

  Scenario: A scalar JSON message is rejected instead of silently ignored
    When the MCP server receives:
      """
      null
      """
    Then the MCP reply at "/error/code" is -32600
    And the MCP reply at "/id" is null

  Scenario: An unsupported JSON-RPC version is rejected
    When the MCP server receives:
      """
      {"jsonrpc":"1.0","id":1,"method":"ping"}
      """
    Then the MCP reply at "/error/code" is -32600

  Scenario: A negative screen index cannot silently select the primary display
    When the MCP server receives:
      """
      {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"capture_screen","arguments":{"screen":-1}}}
      """
    Then the MCP reply at "/result/isError" is true
    And the MCP reply text at "/result/content/0/text" contains "screen"

  Scenario: Tool arguments must be an object
    When the MCP server receives:
      """
      {"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"capture_window","arguments":null}}
      """
    Then the MCP reply at "/result/isError" is true
    And the MCP reply text at "/result/content/0/text" contains "arguments"

  Scenario: Initialize reports the protocol version and server identity
    When the MCP server receives:
      """
      {"jsonrpc":"2.0","id":1,"method":"initialize"}
      """
    Then the MCP reply at "/result/protocolVersion" is "2024-11-05"
    And the MCP reply at "/result/serverInfo/name" is "rustshot"
    And the MCP reply at "/id" is 1

  Scenario: The tool catalogue advertises the four capture tools
    When the MCP server receives:
      """
      {"jsonrpc":"2.0","id":2,"method":"tools/list"}
      """
    Then the MCP reply at "/result/tools/0/name" is "list_screens"
    And the MCP reply at "/result/tools/1/name" is "list_windows"
    And the MCP reply at "/result/tools/2/name" is "capture_screen"
    And the MCP reply at "/result/tools/3/name" is "capture_window"

  Scenario: Ping is answered with an empty result
    When the MCP server receives:
      """
      {"jsonrpc":"2.0","id":3,"method":"ping"}
      """
    Then the MCP reply at "/result" is {}

  Scenario: A notification is not answered
    When the MCP server receives:
      """
      {"jsonrpc":"2.0","method":"notifications/initialized"}
      """
    Then the MCP server does not reply

  Scenario: An unknown method is a JSON-RPC "method not found" error
    When the MCP server receives:
      """
      {"jsonrpc":"2.0","id":4,"method":"resources/list"}
      """
    Then the MCP reply at "/error/code" is -32601
    And the MCP reply text at "/error/message" contains "resources/list"

  Scenario: A malformed line is a parse error against a null id
    When the MCP server receives:
      """
      this is not json
      """
    Then the MCP reply at "/error/code" is -32700
    And the MCP reply at "/id" is null

  Scenario: An unknown tool is a tool error, not a transport error
    When the MCP server receives:
      """
      {"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"reboot"}}
      """
    Then the MCP reply at "/result/isError" is true
    And the MCP reply text at "/result/content/0/text" contains "Unknown tool: reboot"

  Scenario: capture_window without an id is a tool error
    When the MCP server receives:
      """
      {"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"capture_window","arguments":{}}}
      """
    Then the MCP reply at "/result/isError" is true
    And the MCP reply text at "/result/content/0/text" contains "requires an integer 'id'"

  Scenario: capture_window with an unknown window id is a tool error
    The lookup fails long before any file is written: on Windows because no
    window has this id, elsewhere because window capture is Windows-only.
    Either way the client sees a readable tool error, never a crash.

    When the MCP server receives:
      """
      {"jsonrpc":"2.0","id":7,"method":"tools/call","params":{"name":"capture_window","arguments":{"id":18446744073709551615}}}
      """
    Then the MCP reply at "/result/isError" is true

  Scenario: A string request id is echoed back unchanged
    When the MCP server receives:
      """
      {"jsonrpc":"2.0","id":"shot-1","method":"ping"}
      """
    Then the MCP reply at "/id" is "shot-1"

  Scenario: capture_screen with an impossible screen index is a tool error
    This scenario enumerates real displays, so it self-skips on machines with
    no display server at all - the same guard the live-capture tests use. On
    CI a display is always provided (xvfb on Linux, a session on Windows).

    When the MCP server receives on a display-equipped machine:
      """
      {"jsonrpc":"2.0","id":8,"method":"tools/call","params":{"name":"capture_screen","arguments":{"screen":999}}}
      """
    Then the MCP reply reports a tool error mentioning "out of range"
