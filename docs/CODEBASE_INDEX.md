# Codebase Index

Entry point for AI agents working on the Smetana codebase.

| File | Description |
|------|-------------|
| [`docs/01_ARCHITECTURE_OVERVIEW.md`](01_ARCHITECTURE_OVERVIEW.md) | High-level system diagram: layers, data flow from user input through state mutation to rendering |
| [`docs/02_MODULE_MAP.md`](02_MODULE_MAP.md) | Every file with purpose, public exports, and line count, grouped by directory |
| [`docs/03_KEY_TYPES.md`](03_KEY_TYPES.md) | All important structs, enums, traits, and type aliases with fields and locations |
| [`docs/04_PUBLIC_API_SURFACE.md`](04_PUBLIC_API_SURFACE.md) | Every `pub fn` and `pub method` with signature, purpose, and file location |
| [`docs/05_DEPENDENCY_GRAPH.md`](05_DEPENDENCY_GRAPH.md) | Module-level import graph: which modules depend on which, with specific items |
| [`docs/06_STATE_MANAGEMENT.md`](06_STATE_MANAGEMENT.md) | `App` struct fields, editor/project/UI state organization, mutation patterns |
| [`docs/07_BUILD_AND_TEST.md`](07_BUILD_AND_TEST.md) | Build commands, toolchain requirements, dependencies from `Cargo.toml`, platform notes |

**Quick stats:** 32 source files, ~6,300 lines of Rust. Rust edition 2024. Desktop app using eframe/egui.
