[← Back to Index](../index.md)

# Source: Screen Recording Pitfalls

* **Original Title**: Почему запись экрана ломается: путь от кадра до MP4
* **File**: `wiki/raw/003-screen-recording-pitfalls.ru.md`
* **Topic**: Video Encoding, Media Foundation, Pipeline Design.

## Key Takeaways
1. **Time Coordination**: Recording is a temporal system, not just a graphics task.
2. **Native Boundary Risks**: Errors in Media Foundation often manifest as crashes, not clean `Err` results.
3. **Pipeline Stability**: Resilience requires handling dropped frames, dimension constraints, and clean stop semantics.
4. **Happy Path Fallacy**: Recording quickly exposes missing edge-case handling in the capture pipeline.

## Linked Concepts
* [Screen Recording](screen-recording.md)
