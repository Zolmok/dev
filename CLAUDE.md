# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## Project Overview

`dev` is a Rust CLI tool for creating and managing tmux or Zellij sessions for development projects. It reads project-specific configuration files to set up preconfigured terminal layouts.

## Commands

### Build
```sh
cargo build --release
```

### Run
```sh
cargo run              # Run with debug build
./target/release/dev   # Run release binary
dev --init             # Generate .dev.json config in current directory
```

### Test
```sh
cargo test             # Run all tests
cargo test <test_name> # Run specific test
```

## Architecture

### Single-file structure
All code is in `src/main.rs`, organized into sections:
- **Core Types**: `Args`, `App` structs
- **Traits**: `ProcessRunner`, `FileSystem`, `Environment` for dependency injection
- **Real Implementations**: `RealProcessRunner`, `RealFileSystem`, `RealEnvironment`
- **Error Types**: `DevError` with conversions from `io::Error` and `serde_json::Error`
- **Configuration**: `Window`, `Config` structs with serde derive
- **App Builders**: Pure functions that create tmux `App` commands
- **Core Functions**: Functions that use injected dependencies
- **Tests**: Mock implementations and comprehensive unit tests

### Dependency injection pattern
The codebase uses traits for testability:
- `ProcessRunner` - abstracts process execution
- `FileSystem` - abstracts file operations
- `Environment` - abstracts environment queries (cwd)

Tests use mock implementations (`MockProcessRunner`, `MockFileSystem`, `MockEnvironment`).

### Configuration priority
1. `.dev.json` - JSON config for tmux
2. `.dev.kdl` - KDL layout for Zellij
3. Default layout - 3 windows (Claude-Code, Zsh, Server)

## Dependencies

- `serde` - serialization/deserialization with derive macros
- `serde_json` - JSON parsing
