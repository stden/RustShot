[← Back to Index](../index.md)

# Source: Screenshot Architecture

* **Original Title**: Архитектура скриншот-утилиты: маленькая программа, много скрытой сложности
* **File**: `wiki/raw/002-screenshot-architecture.ru.md`
* **Topic**: Software Architecture, Modular Design, State Management.

## Key Takeaways
1. **Module Separation**: Architecture is divided into specialized layers (`capture`, `selection`, `record`, `tray`).
2. **State Balance**: Program manages hidden states (tray, selection, recording flags).
3. **Engineering Paradox**: Minimal interfaces require more internal structure to avoid chaos.
4. **Boundary Discipline**: High reliance on external environment (FS, displays, registry) necessitates rigorous input/boundary validation.

## Linked Concepts
* [Architecture Overview](architecture-overview.md)
