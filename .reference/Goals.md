## Goals
- A native, portable (macOS / Windows / Linux), fully open-source layered screenplay document editor, roughly matching Final Draft's feature surface over time.
- The UI can be swapped without touching the core: egui today, maybe Slint, Qt or something else later.

## Principles
- Engine-first, headless-first. Every feature is reachable without a GUI: from tests, the CLI and MCP. The GUI is one client of the engine.
- Everything is a command. Each user-visible action has a stable CommandId (such as filter.blur.gaussianBlur) with typed, serializable parameters. The menu, command palette, shortcuts, recorded actions/macros, CLI, MCP and plugins all dispatch the same commands.