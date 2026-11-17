# Requirements Document

## Introduction

RustShot is a lightweight screenshot and screen recording utility written in Rust. The application provides system tray integration with hotkeys, interactive region selection via a fullscreen overlay, CLI mode for scripted capture, and screen recording to MP4 on Windows. This document specifies the complete set of requirements covering the current feature set and its planned evolution across all modules: capture, selection, recording, tray, settings, CLI, and application lifecycle.

## Glossary

- **RustShot_App**: The main RustShot binary application, responsible for mode routing (tray, CLI, selection) and lifecycle management
- **Capture_Engine**: The screenshot capture subsystem (`capture.rs`) that resolves screens, captures pixels, converts formats, and saves files
- **Selection_Overlay**: The fullscreen borderless overlay window (`selection.rs`) that composites the virtual desktop and lets the user drag-select a region
- **Recording_Engine**: The screen recording subsystem (`record.rs`) that uses Windows Media Foundation to encode H.264 MP4 video from captured frames
- **Tray_Manager**: The system tray icon and context menu subsystem (`tray.rs`) that provides the desktop integration surface
- **Settings_Store**: The JSON-based settings persistence subsystem (`settings.rs`) that loads, saves, and normalizes user preferences
- **CLI_Handler**: The command-line interface subsystem (`cli.rs`) that parses arguments and routes single-capture operations
- **Region**: A rectangular screen area defined by origin (x, y) and dimensions (width, height), used for cropping screenshots and bounding recordings
- **Virtual_Desktop**: The composited image of all connected monitors stitched into a single coordinate space
- **HNS**: Hundred-nanosecond units used by Windows Media Foundation for frame timestamps (1 second = 10,000,000 HNS)
- **BGRA_Bottom_Up**: The pixel format required by the Media Foundation H.264 encoder — Blue-Green-Red-Alpha channel order with rows stored bottom-to-top
- **Stop_Flag_File**: A filesystem-based IPC mechanism where the parent tray process writes a file to signal the child recording process to stop

## Requirements

### Requirement 1: Screenshot Capture via CLI

**User Story:** As a developer, I want to capture screenshots from the command line with explicit options, so that I can automate screenshot workflows without GUI interaction.

#### Acceptance Criteria

1. WHEN the user runs RustShot with `--cli` flag, THE CLI_Handler SHALL capture a screenshot of the default screen and save it to the default output directory
2. WHEN the user provides `--output <path>`, THE Capture_Engine SHALL save the screenshot to the specified path exactly as given
3. WHEN the user provides `--format <fmt>` with a supported format (png, jpg, bmp), THE Capture_Engine SHALL encode the screenshot in the requested format
4. WHEN the user provides `--screen <index>`, THE Capture_Engine SHALL capture the screen at the specified index
5. WHEN the user provides `--list`, THE CLI_Handler SHALL print a list of available screens with their index and resolution, and exit without capturing
6. WHEN the user provides `--window`, THE RustShot_App SHALL capture the currently active window (Windows only)
7. IF the specified screen index is out of range, THEN THE Capture_Engine SHALL return an error message containing the requested index and the valid range
8. IF no screens are detected, THEN THE Capture_Engine SHALL return the error "No screens detected"
9. WHEN the user provides `--output` with a known image extension and no explicit `--format`, THE CLI_Handler SHALL infer the format from the output file extension
10. WHEN the user provides both `--format` and `--output`, THE CLI_Handler SHALL use the explicit `--format` value regardless of the output file extension

### Requirement 2: Screenshot File Output

**User Story:** As a user, I want screenshots saved with predictable names and paths, so that I can find and organize captured images.

#### Acceptance Criteria

1. THE Capture_Engine SHALL save screenshots to the `Pictures/RustShot/` directory by default
2. THE Capture_Engine SHALL generate filenames with the pattern `screenshot_YYYYMMDD_HHMMSS_NNNNNNNNN.<ext>` using nanosecond-precision timestamps
3. WHEN an explicit output path is provided, THE Capture_Engine SHALL use that path verbatim without appending a timestamp or changing the extension
4. THE Capture_Engine SHALL create parent directories recursively if the output path does not exist
5. WHEN `ensure_parent_directory` is called multiple times on the same path, THE Capture_Engine SHALL succeed on every call (idempotent operation)
6. THE Capture_Engine SHALL generate filenames containing only alphanumeric characters, underscores, and dots

### Requirement 3: Image Format Support

**User Story:** As a user, I want to save screenshots in PNG, JPG, or BMP format, so that I can choose the format that fits my workflow.

#### Acceptance Criteria

1. THE Capture_Engine SHALL support three output formats: PNG, JPG, and BMP
2. THE Capture_Engine SHALL use PNG as the default format
3. WHEN saving as JPG, THE Capture_Engine SHALL convert RGBA pixel data to RGB before encoding
4. WHEN `Format::extension()` is called, THE Capture_Engine SHALL return a lowercase string without a leading dot ("png", "jpg", or "bmp")
5. WHEN `Format::to_image_format()` is called, THE Capture_Engine SHALL map to the corresponding `image` crate format variant (Png, Jpeg, Bmp)
6. THE Capture_Engine SHALL produce distinct `ImageFormat` values for each of the three supported formats

### Requirement 4: Multi-Monitor Screenshot Support

**User Story:** As a user with multiple monitors, I want to capture any screen or select a region across my desktop, so that I can screenshot any part of my workspace.

#### Acceptance Criteria

1. WHEN no screen index is specified, THE Capture_Engine SHALL default to screen index 0
2. WHEN a valid screen index is specified, THE Capture_Engine SHALL capture that specific screen
3. IF the screen index exceeds the number of available screens, THEN THE Capture_Engine SHALL return an error containing the index and the valid range "0-N"
4. WHEN `find_screen_index_at_position` is called with coordinates inside a monitor, THE Selection_Overlay SHALL return the index of that monitor
5. WHEN `find_screen_index_at_position` is called with coordinates outside all monitors, THE Selection_Overlay SHALL return `None`
6. THE Selection_Overlay SHALL treat monitor right and bottom boundaries as exclusive (a point at x=1920 belongs to the adjacent monitor, not the one ending at 1920)
7. THE Selection_Overlay SHALL support monitors with negative coordinate origins (monitors positioned to the left of or above the primary monitor)

### Requirement 5: Interactive Region Selection

**User Story:** As a user, I want to visually select a screen region with my mouse, so that I can capture exactly the area I need.

#### Acceptance Criteria

1. WHEN the selection overlay is activated, THE Selection_Overlay SHALL capture all screens into a single Virtual_Desktop composite image
2. THE Selection_Overlay SHALL display the composite as a fullscreen borderless always-on-top window via softbuffer CPU rendering
3. WHILE the user has not started dragging, THE Selection_Overlay SHALL display a red crosshair at the cursor position
4. WHILE the user is dragging, THE Selection_Overlay SHALL display a 3-pixel cyan border around the selection rectangle
5. WHEN the user releases the mouse button after dragging, THE Selection_Overlay SHALL crop the selected region from the composite image
6. IF the selected region is 5x5 pixels or smaller, THEN THE Selection_Overlay SHALL reject the selection and return `None`
7. WHEN the selected region is 6x6 pixels or larger, THE Selection_Overlay SHALL accept the selection and return a valid Region
8. WHEN the user right-clicks or presses Escape, THE Selection_Overlay SHALL cancel the selection without modifying the clipboard
9. WHEN `build_region_from_drag` is called with reversed start/end coordinates (drag from bottom-right to top-left), THE Selection_Overlay SHALL normalize the coordinates to produce a valid Region with positive width and height
10. THE Selection_Overlay SHALL support region selection across negative coordinate spaces (multi-monitor setups with monitors to the left of origin)

### Requirement 6: Clipboard Integration

**User Story:** As a user, I want selected screenshots automatically copied to my clipboard, so that I can paste them immediately into other applications.

#### Acceptance Criteria

1. WHEN a region is successfully selected, THE RustShot_App SHALL copy the cropped RGBA image to the system clipboard
2. IF the selection is cancelled, THEN THE RustShot_App SHALL leave the clipboard unchanged and return the error "Selection cancelled"
3. IF the selection fails with an error other than cancellation, THEN THE RustShot_App SHALL propagate the error prefixed with "Selection error: "
4. IF copying to the clipboard fails, THEN THE RustShot_App SHALL propagate the clipboard error
5. WHEN `selection_stderr_message` is called with "Selection cancelled", THE RustShot_App SHALL return `None` (suppress stderr output for cancellation)
6. WHEN `selection_stderr_message` is called with any other error, THE RustShot_App SHALL return `Some` containing the error message

### Requirement 7: System Tray Integration

**User Story:** As a desktop user, I want RustShot to live in my system tray with a context menu, so that I can quickly access capture and recording features.

#### Acceptance Criteria

1. WHEN RustShot starts in tray mode (no CLI flags), THE Tray_Manager SHALL create a system tray icon with a 32x32 RGBA camera glyph
2. THE Tray_Manager SHALL provide a context menu with items: Capture, Capture Window, Open Folder, Settings, Start/Stop Recording, and Exit
3. WHEN the user selects "Capture" from the tray menu, THE RustShot_App SHALL spawn a child process with `--select` to perform region selection
4. WHEN the user selects "Capture Window", THE RustShot_App SHALL capture the currently active window (Windows only)
5. WHEN the user selects "Open Folder", THE RustShot_App SHALL open the screenshots directory in the system file manager
6. WHEN the user selects "Exit", THE RustShot_App SHALL shut down cleanly
7. THE Tray_Manager SHALL generate an icon where every pixel has alpha of either 0 (transparent) or 255 (opaque)
8. THE Tray_Manager SHALL render the camera body in RGB(41, 128, 185) (dark blue) and the lens in white (255, 255, 255)
9. THE Tray_Manager SHALL produce an icon with horizontal symmetry

### Requirement 8: Global Hotkey Support

**User Story:** As a user, I want keyboard shortcuts to trigger screenshots and recordings, so that I can capture content without navigating menus.

#### Acceptance Criteria

1. WHILE RustShot is running in tray mode, THE RustShot_App SHALL register PrintScreen as the global hotkey for screenshot capture
2. WHILE RustShot is running in tray mode, THE RustShot_App SHALL register Ctrl+Shift+R as the global hotkey for toggling screen recording
3. WHEN PrintScreen is pressed, THE RustShot_App SHALL spawn a selection child process identical to the "Capture" tray menu action
4. WHEN Ctrl+Shift+R is pressed while not recording, THE RustShot_App SHALL start a recording session after region selection
5. WHEN Ctrl+Shift+R is pressed while recording, THE RustShot_App SHALL stop the active recording session

