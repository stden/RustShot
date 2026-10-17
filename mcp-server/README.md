# RustShot MCP Server

RustShot does not ship an MCP server yet.

This directory documents the planned MCP integration so the repository shape is
clear, but the current binary has no `--mcp` flag and no MCP request loop.

## Current status

- Implemented today: tray mode, CLI capture mode, selection subprocesses, and
  Windows recording support.
- Not implemented yet: MCP server mode, MCP tool dispatch, and `rustshot --mcp`.
- Repository `.mcp.json` is intentionally empty until the server exists.

## Planned tool surface

These are the tools the future MCP server is expected to expose:

| Tool | Description |
|------|-------------|
| `screenshot` | Capture a screenshot (full screen or specific monitor). Returns the file path. |
| `screenshot_region` | Interactive region selection, copies to clipboard, and returns the outcome. |
| `list_screens` | List available monitors with resolution info. |
| `start_recording` | Start screen recording (Windows only). Returns session info. |
| `stop_recording` | Stop an active recording session. Returns the MP4 file path. |
| `open_screenshots_folder` | Open the screenshots directory in the file manager. |

## Planned setup

Once MCP support is implemented, the expected project config will look like:

```json
{
  "mcpServers": {
    "rustshot": {
      "command": "rustshot",
      "args": ["--mcp"],
      "env": {}
    }
  }
}
```

Do not enable that config yet. The current binary will fail because `--mcp`
does not exist.

## Implementation plan

1. Add `--mcp` CLI flag to the `Args` struct in `src/main.rs`.
2. Create `src/mcp.rs` to host the MCP JSON-RPC request loop.
3. Expose each planned tool with a stable request and response schema.
4. Route tool calls to existing `capture.rs`, `selection.rs`, `record.rs`, and
   shared helpers in `main.rs`.
5. Add tests for initialization, tool listing, dispatch, and error mapping.
