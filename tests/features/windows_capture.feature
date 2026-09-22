@windows @mcp_capture_screen @mcp_capture_window
Feature: Windows MCP capture transport
  Scenario: The MCP process returns real screenshots and window images
    When MCP captures screen zero and an owned changing window over stdio
    Then both captures return saved files and inline previews in the requested formats
